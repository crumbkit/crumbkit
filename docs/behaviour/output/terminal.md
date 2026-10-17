# Terminal detection

This page covers what changes when a stream is a terminal, how a tool turns
terminal detection off, and how crumbkit tests that a run at a terminal
prints what a piped run prints.
[`Guide::without_terminal_detection`](../../../crates/crumbkit/src/help/terminal.rs)
describes the setting itself.

## What detection changes

A tool's result, failure or note prints the same bytes whether a stream is a
terminal or a pipe, and whether or not a coding agent runs the tool
(OUTPUT-1). An agent, a script and a person then read the same output, and a
golden file of a piped run also pins what a person sees at a terminal.

Detection changes only these:

- Help and the version keep clap's colours when standard output is a
  terminal ([help](../help/README.md)).
- `Output::progress_allowed` is true only when `--json` is off and standard
  error is a terminal ([output](README.md#following-the-callers-format)).
- A picker and `Output::confirm` ask only when standard input and standard
  error are terminals and no agent is detected
  ([effects](../effects/README.md), [pickers](../effects/pickers.md)).
- The usage log's `caller` field reads detection. The usage log is not
  output that a caller reads ([usage log](../usage-log/README.md)).

Why: ADR-0049.

## Turning detection off

A tool turns detection off with `Guide::without_terminal_detection`, which
takes the tool's reason. crumbkit-test's `check_opt_outs` lists the setting
as `terminal-detection/off` with that reason and two costs: a person at a
terminal is never asked and must type every argument and `--yes`, and
progress and help colour never show ([opt-outs](../adoption/opt-outs.md)).
The `Output` that `parse` returns, the `Output` inside a `Stop` and the
`Output` that `Output::new` builds copy the setting from the guide, as they
copy the guide's JSON layout and code renames.

The setting belongs to the tool and has no environment variable. A caller
already turns colour off with `NO_COLOR` or `TERM=dumb`, which clap reads,
questions off with `--no-input`, and progress off with `--json`.

Why: ADR-0049.

## The pseudo-terminal test

shelf's `tests/terminal.rs` runs each of shelf's examples with standard input
and standard output on one pseudo-terminal and standard error on another. It
compares the bytes of each stream with the golden files of the piped runs:
standard output with the `.txt` file, and standard error with the `.stderr`
file, or with nothing when the example has none
([golden files](../checks/golden-files.md)). It first checks that help at the
pseudo-terminal has clap's colours, so the test fails when the run does not
reach a terminal.

The test uses rustix's safe `pty` and `termios` functions, which need no
`unsafe` code (RUST-2) and add no crate to the lock file. It runs on macOS
and Linux only, because Windows has no such API without `unsafe` code or a
new crate.

Why: ADR-0049.
