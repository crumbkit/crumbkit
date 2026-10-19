# ADR-0029. Usage log

- Date: 2026-10-10
- Status: amended by ADR-0095

## Context

A tool's builder wants to know where callers waste effort, such as
refused flags, help read before a command and listings cut short. Agents
run tools many times a session, so these patterns repeat. Recording them
must never leak what callers typed, never change a tool's output or exit,
and never cost a tool that does not want it.

## Decision

- The usage log appended one JSON line per run to a file on the user's
  machine, recording the shape of the run and never the values the caller
  typed. `self usage` reads it (ADR-0034).
- The log sat behind the `usage-log` feature, off by default, so a tool
  without it compiles none of its code or dependencies. Starting the log
  is one line in `main`, which keeps the opt-in explicit. The
  platform-specific calls live in one module with a Unix and a Windows
  half.
- On 2026-10-14 the log moved from that feature to the published
  crumbkit-usage crate, which depends on crumbkit. A tool that does not
  depend on crumbkit-usage compiles none of the log's code or
  dependencies, and a tool that does starts the log with
  `crumbkit_usage::start()`.
- `start` took no arguments, and `parse` gives the recorder the tool's
  name and version, so `start` never builds the command tree. A run that
  ends before `parse` writes no line, because it has no tool name.
- crumbkit's entry points reported the run's shape to the observer
  (ADR-0057): the command path, the flags typed by name, help requests,
  refusals and clap's nearest flag, the exit code, the code in the tool's
  spelling, the bytes written and whether a listing was cut or empty.
  `ended` records a tool's real exit number, because `ExitCode` cannot be
  read back as a number, and writes the line at once, so a later
  `process::exit` keeps it.
- The record's types enforced the privacy rule inside crumbkit: every
  name is built from a `clap::Command`, a `clap::Arg` or a `Code`, the
  command line appears only as a keyed hash, and an end-to-end test plants
  marker values and fails when any line holds them. The tool's own text
  and its `ext` section are its responsibility.
- `session` and `args` were keyed SipHash-2-4 hashes, so nobody without
  the key can test a guessed command line against a hash, and no further
  crate was needed. The key file is linked into place with `hard_link`, so
  it either holds a whole key or does not exist.
- The log lived in the XDG state directory, or `%LOCALAPPDATA%` on
  Windows, which never roams, with private modes on Unix.
- A run appended while holding a lock on a separate `usage.lock` file,
  because a lock belongs to the opened file and a trim replaces the log.
  The lock wait is bounded, a line over 1 KiB is dropped, and an oversized
  log is trimmed to its newest lines through a temporary file and a
  rename, so a run that ends mid-trim leaves the old log or the new one.
- A signal thread wrote the line with `"interrupted": true` on SIGINT or
  SIGTERM and then let the signal take its default effect, so the process
  ends as it would have without the log. Waits on both sides keep the
  line whole. SIGHUP is left alone, so a run under `nohup` behaves as it
  would without the log. A tool with its own handler starts with
  `Signals::Tool`.
- Recording never changes the tool's output or exit code, and every
  failure to write is silent.

## Consequences

crumbkit's ban on unsafe code set several limits: a process that started
with SIGINT ignored now ends on it, Windows sees only Ctrl-C, Windows
files inherit the access control list of `%LOCALAPPDATA%`, and a Windows
run outside Windows Terminal shares one session. `docs/behaviour/usage-log/README.md`
describes the current record and file handling.

Considered: an `ended` that only records and leaves the line to the guard, no listener when the tool owns the signals, with a separate call that reports the signal, a trim that truncates the log and writes the kept lines in place, a lock on the log itself that a run checks against the file at the path after it takes the lock, a key file created with `create_new` and then written, HMAC-SHA256 with a key from getrandom, a log handle passed to `parse` and `finish`, a `start` that takes the tool's command, a signal thread that also waits for SIGHUP, the parent process id on Windows through `windows-sys`, an access control list set on the Windows log, the roaming `%APPDATA%` for the Windows log, counting every positional value as ids, a session from the agent's id only, a log that ships off the machine, a record per refused flag, the log behind a `usage-log` cargo feature of crumbkit.
