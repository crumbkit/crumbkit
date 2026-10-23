# Usage log

The usage log appends one JSON line per run of a tool to a file on the
user's machine. The published crumbkit-usage crate holds it. This page
covers how a tool turns the log on, what each run
reports to it, what the line holds and never holds, where the file lives,
and how the log behaves under concurrent runs and signals. The
[trace context](trace-context.md) page covers the trace ids in the line, and
the [usage report](report.md) page covers `self usage` and
`self figures list`, which read the log.

## The crate and its platforms

The log is the crumbkit-usage crate, which depends on crumbkit. A tool that
does not depend on crumbkit-usage compiles none of the log's code and none
of its dependencies. crumbkit-usage's
[`Cargo.toml`](../../../crates/crumbkit-usage/Cargo.toml) lists the crates
that it adds and the reason for each. The crate builds on Unix and
Windows, and a build for any other target fails with a compile error. The
calls that differ between the two platforms live in
[`platform.rs`](../../../crates/crumbkit-usage/src/platform.rs), which
has a Unix half and a Windows half, and the rest of the log has no `cfg` of
its own.

Why: ADR-0029, ADR-0003.

## Starting the log

A tool depends on crumbkit-usage and adds one line to `main`:
`let _log = crumbkit_usage::start();`. The rustdoc of
[`start`](../../../crates/crumbkit-usage/src/recorder.rs) states what
`start` does, what the guard does when it drops, and when a run installs no
recorder, such as a second `start` in one process or a run that answers a
shell's [completion request](../parsing/completion.md). `start` installs the
log as one of the process's [run observers](../run-observer/README.md) with
`crumbkit::events::observe`, beside any observer that the tool installed
before it.

`start` never builds the tool's command tree. `parse` gives the recorder the
tool's name and version from the command it parses against, so a run that
ends before it reaches `parse` writes no line.

Why: ADR-0029, ADR-0057.

## What each part of a run reports

- `parse`, `parse_from` and `parse_to` report the command path and the long
  name of each flag the caller typed, read from clap's value source and
  including global flags, or the short name such as `-q` for a flag without
  a long one.
- They report a request for help as `short` for `-h` and `long` for
  `--help` or the `help` command, with the command path that the help names.
- For a refused command line they report how many flags name no flag of the
  command, clap's suggested real flag, the exit code of a refused command
  line, and the code `<tool>::cli::usage` in the tool's spelling or the
  tool's rename of it.
- For a parsed command line they report how many values the command's
  positional arguments with the value name `ID` received, and, as `picked`,
  how many arguments a [picker](../effects/pickers.md) filled, without the
  picked values.
- `finish` and `finish_to` report the exit code as the number they return,
  including a tool's code that the guide declares for the run's first
  failure, the code of a failure or refusal in the tool's spelling, the bytes
  written to standard output, and whether a listing left out rows of its
  source, as one that shows fewer rows than its total or comes from a
  capped source does. A batch reports the code of its first failure.
- A [stream](../output/streams.md) reports its run as `finish` does.
- A run that never reaches `finish` or a stream's end, such as a child
  process or the tool's own exit path, reports its exit with
  [`events::ended`](../../../crates/crumbkit/src/events.rs). The line records
  the tool's real exit number, such as 124, because `ExitCode` cannot be
  read back as a number. `ended` returns that number as the `ExitCode` for
  `main` and writes the line at once, so a tool that then exits through
  `std::process::exit` still has its line.
- A run that reports a failure and carries on past it reports the code with
  [`events::tolerated`](../../../crates/crumbkit/src/events.rs). The line's
  `tolerated` field lists the first 8 distinct codes in the tool's spelling,
  in the order reported, and drops later codes, so the line stays within its
  size limit.
- crumbkit-aliases' `expand` calls
  [`events::aliased`](../../../crates/crumbkit/src/events.rs) when it replaces
  an [alias](../parsing/aliases.md). The line then records `"alias": true`
  and the expanded command. crumbkit-aliases has no feature of its own.
- `ended`, `tolerated` and `aliased` live in `crumbkit::events`, exist in
  every build and do nothing without an installed observer.

crumbkit sends each report to the [run observer](../run-observer/README.md),
and the log's observer passes it to the recorder. Every report does nothing
when no observer or no recorder is installed. A report is a load of the
observer, a lock of a static `Mutex` and a check of its state, and only `parse` copies the
words of the command line, once.

Why: ADR-0029, ADR-0037, ADR-0041, ADR-0043, ADR-0057.

## The record

The line is the `<tool>/usage/v1` record, whose schema is checked in as
[`usage-v1.json`](../../../crates/crumbkit-usage/schemas/usage-v1.json). The schema
lists every field and what it holds, and its `schema` field accepts any
tool's `<tool>/usage/v1`, such as `shelf/usage/v1`. A field that does not apply to the run
is left out. `trace`, `run` and `parent` place the run in a W3C trace, as
[trace context](trace-context.md) describes. The `empty` and `facts` fields
are optional, and a record with them keeps the `v1` id. Every field other
than `facts` is one that crumbkit writes and checks.

Why: ADR-0029, ADR-0007, ADR-0095.

## The tool's own facts

A tool records facts that crumbkit cannot see, such as the requests it sent
to a server and the time it waited for them, under the record's `facts`
field. The tool keeps them in a type of its own that implements
[`Extension`](../../../crates/crumbkit-usage/src/facts.rs), and changes that
value during the run with `crumbkit_usage::extend`. Without a started log,
`extend` does nothing. When the line is written, each extension writes its
facts through `Facts`, which takes three kinds:

- a count under a `Key`, such as `"counts": {"requests": 3}`;
- a `Label` under a `Key`, such as `"labels": {"route": ["/books/{id}"]}`;
- the wall-clock time that the run spent blocked waiting on another
  system, such as `"waited_ms": 1520`. Overlapping waits count once: a tool
  that sends several requests at once records the time from the first
  request's start to the last one's end, so the figure never passes the
  run's wall time. Several calls to `Facts::waited` add.

A `Key` is 1 to 32 bytes of lowercase letters, digits and `_`, starting
with a letter. A `Label` is 1 to 64 bytes of ASCII letters, digits, braces
and `_-./:`, so it holds a route template, a status class or a kind of
target, and no space, query string or `@`. Both hold a `&'static str` and
are built by a `const fn`, so a tool declares them as constants and a bad
one fails to compile. A run may keep several extensions, such as the
tool's own and one of a library it uses: counts under one key add, labels
under one key are kept once each in the order written, and waiting times
add. A count of 0 is left out, and a section with nothing in it is left
out.

The facts count towards the line's limit of 1 KiB. A line that would pass
it with its facts is written without them. The closure passed to `extend`
runs while the log holds its lock, so a call to `extend` or a crumbkit
report that it makes on the same thread is skipped without a word. The
closure must be short and must not wait on other threads: on SIGINT or
SIGTERM the log waits at most 500 ms for its lock, and then lets the
signal end the process without a line. An extension's `write` runs when
the line is written, which can be on the thread that answers the signal,
and nothing bounds it, so it must be short and must not wait on other
threads too.

A closure or a `write` that panics leaves the line without facts, and the
line is still written. The panic message is printed to standard error.
When panics unwind, the run keeps its exit code and a signal still ends
the process. Under `panic = "abort"`, or with a panic hook that ends the
process, the panic ends the run instead.

shelf's `books show` records how many books it looked up and how many were
missing.

Why: ADR-0144.

## What the record never holds

The record holds no argument values, URLs, ids, paths, error text or
messages. Inside crumbkit, its types enforce this. Every name in the record
is built from a `clap::Command` for its name, its version or a path of its
subcommands, from a `clap::Arg` for its long or short flag name, or from a
crumbkit `Code` in the tool's `Spelling`, so no crumbkit code can put a word
that the caller typed into a name. The counts are numbers, the trace ids are
hex that crumbkit generated or validated, and the command line appears only
as a keyed hash.

The types do not check the text that the tool itself gives its commands,
arguments, version and codes. In the tool's facts, the types keep text to
the keys and labels that the tool declares as `&'static str` constants,
and they cannot stop a tool from writing a value, such as an id, as a
count.

shelf's end-to-end test in
[`tests/usage.rs`](../../../examples/shelf/tests/usage.rs) plants
`s3cr3t-<field>-PLANTED` in option values, positionals, a command path, an
unknown flag's name and its value, a help topic, an unknown command,
`TRACEPARENT` and `TRACESTATE`, and fails when any line of the log holds
`PLANTED`.

Why: ADR-0029.

## Caller and session

`caller` is the id that is-ai-agent gives for the coding agent that ran the
tool, such as `claude-code`, read through the same detection that decides
whether a person can answer a question
([terminal detection](../output/terminal.md)). Without an agent, `caller`
is `person` when standard input is a terminal and `unknown` otherwise.

`session` is the keyed hash of the agent's session id, or of the parent
process id when the agent gives none, so the runs of one session or one
shell share a value. On Windows the fallback is the Windows Terminal tab's
`WT_SESSION`, or the text `windows` outside Windows Terminal, because
reading the parent process id there needs unsafe code. Outside
Windows Terminal, every run without an agent shares one session.

Why: ADR-0029, ADR-0043.

## Hashes and their key

`session` and `args` are keyed SipHash-2-4 hashes, written as 16 lowercase
hex digits. `args` hashes every word after the tool's name, each with its
length, so identical command lines share a hash and different ones almost
never do.

The key is 16 random bytes from `/dev/urandom`, or from getrandom on
Windows, kept in `usage.key` beside the log with mode 0600 on Unix.

- The first run writes the key to a temporary file in the same directory,
  syncs it, and links it into place as `usage.key` with `hard_link`, which
  fails when the name exists. `usage.key` therefore either holds a whole key
  or does not exist, and when two first runs race, the loser reads the
  winner's key.
- A `usage.key` of the wrong length, such as one left by an older version,
  is replaced with a new key through `rename`, so the run and later runs
  write their lines again.
- On a Windows file system without hard links, such as FAT, `hard_link`
  fails, and every run drops its line without a word.

Why: ADR-0029.

## The log file

The log is `<state dir>/<tool>/usage.jsonl`. On Unix the state directory is
`$XDG_STATE_HOME`, or `~/.local/state` when that variable is unset, empty or
relative, as etcetera gives it. On Windows the state directory is
`XDG_STATE_HOME` when it is absolute and otherwise `%LOCALAPPDATA%`, which is
local to the machine and never roams.

crumbkit creates the directory with mode 0700, and the log and its lock file
`usage.lock` with mode 0600. On Windows the directory and files inherit the
access control list of `%LOCALAPPDATA%`, which admits the user, the
administrators and the system. crumbkit leaves that list unchanged, because
setting a narrower one needs unsafe code.

Why: ADR-0029.

## Appending, locking and trimming

- A run appends its line while it holds the lock on `usage.lock`, which it
  takes with `File::try_lock`. It retries for at most 100 ms, and when the
  lock is still held it drops its line. The lock has a file of its own,
  which is never renamed.
- A line longer than 1 KiB is dropped.
- When the log is over its size limit, the run keeps its newest whole lines
  that fit in half of the limit and then appends its own, while it holds the
  lock, so at least half the limit of new lines goes in before the next
  trim. [`file.rs`](../../../crates/crumbkit-usage/src/file.rs) states the
  sizes: a 10 MB limit and 5 MB kept.
- The trim writes the kept lines to a temporary file in the same directory,
  syncs it, closes its handle on the old log and renames the new file over
  `usage.jsonl`, so a run that ends during a trim leaves either the old log
  or the new one.
- A last line without a newline is a write that ended early. A trim drops
  it, and the next append starts on a new line, so the torn line never joins
  the next record.
- Each run writes at most one line. Recording never changes the tool's
  output or exit code: every failure to write is silent, and a poisoned lock
  counts as no recorder.

Why: ADR-0029.

## Signals on Unix

The signal thread waits for SIGINT and SIGTERM. On a signal, the thread
writes the line with `"interrupted": true` and then lets the signal take its
default effect, so the process ends as it would have without the log.

- When the main thread is already writing the line, the signal thread waits
  a bounded time for that write to finish before the signal takes effect.
- When the signal thread is already writing the line as `main` ends, the
  guard waits for that write in the same way before `main` returns.
- The bound covers the lock wait and a trim on a slow disk.
  [`recorder.rs`](../../../crates/crumbkit-usage/src/recorder.rs) states it
  as 500 ms, which is also the longest a person who presses Ctrl-C waits for
  the tool to end.
- A process that started with SIGINT or SIGTERM ignored ends on that signal
  once the log has started, such as a background job of a non-interactive
  shell, which starts with SIGINT ignored. crumbkit forbids unsafe code,
  and reading a signal's current action needs `sigaction`, which is unsafe.
- SIGHUP is left alone, so a run under `nohup` behaves as it would without
  the log.

Why: ADR-0029.

## Signals on Windows

On Windows the signal thread waits for SIGINT alone, which the C runtime
raises on Ctrl-C. A signal-hook flag records the signal, and the thread
checks it every 50 ms. After the line, the thread ends the process with
`STATUS_CONTROL_C_EXIT`, the exit code that Ctrl-C gives a process without a
handler. Ctrl-Break, closing the console, logging off and shutting down end
the process with no line, because the C runtime raises no SIGINT for them
and a console control handler for them needs unsafe code.

Why: ADR-0029.

## A tool that handles its own signals

The log's thread always handles SIGINT and SIGTERM: it writes the line with
`"interrupted": true` and lets the signal end the process. A handler that
the tool installs through signal-hook runs beside the thread, which still
ends the process. A handler that replaces the signal's action directly, as
the ctrlc crate does, removes crumbkit's listener, and the line then
records the exit without `interrupted`. A tool that must keep running
after Ctrl-C or SIGTERM keeps its own handler and runs without the log. The
rustdoc of [`start`](../../../crates/crumbkit-usage/src/recorder.rs) says
so.

Why: ADR-0029, ADR-0095.

## Child processes

crumbkit tells the observers when a command hands the terminal to a child
and, on Windows, when the child exits. Once a command has handed the
terminal to a
[child process](../output/child-processes.md), the signal thread skips
SIGINT for the rest of the run on Unix, and while the child runs on
Windows, so Ctrl-C ends only the child. A child that a signal ended marks
the line `"interrupted": true`, and the command reports its exit through
`ended`.

Why: ADR-0029, ADR-0042, ADR-0057.

## Cut-short and empty results

- A listing that the [output budget](../output/budget/README.md) cut short, and a
  stream that stopped at the budget, mark the line `"truncated": true` with
  `code` set to the over-budget code. A cut listing still exits with 0.
- A [listing](../output/listings/README.md) that left rows out because of
  `--limit`, or because of a source that the tool did not count, also marks
  the line `"truncated": true`, without a code.
- A listing or stream that had no rows and printed the empty note marks the
  line `"empty": true`.

Why: ADR-0029, ADR-0046, ADR-0050.
