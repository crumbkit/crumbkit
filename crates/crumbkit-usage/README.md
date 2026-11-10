# crumbkit-usage

crumbkit-usage keeps a local usage log for a tool built with crumbkit. The
log is one JSON line per run in a file on the user's machine. The line
records the shape of the run, such as the command path, the flags typed and
the exit code, and never the values that the caller typed. The tool's
`self usage` and `self figures list` commands read the log back.

## Adding it

A tool adds crumbkit-usage beside crumbkit:

```toml
[dependencies]
crumbkit-usage = "0.3"
```

The crate builds on Unix and Windows.

## Example

A tool's `main` calls [`start`] first and keeps the guard that it returns
until `main` returns:

```rust,no_run
use std::process::ExitCode;

fn run() -> ExitCode {
    // parse, run the command and finish, as crumbkit's README shows.
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let _log = crumbkit_usage::start();
    run()
}
```

`start` installs the log as crumbkit's run observer, so crumbkit's `parse`,
`finish`, streams and child hand-overs tell the log what the run did, and
the guard writes the line when it drops. A tool that installs its own
observer keeps it, and both observers hear each report. The log's own
thread writes the line when SIGINT or SIGTERM ends the run. A tool whose own
code ends it on a signal calls [`start_under`] in place of `start`, which
starts no thread and hands that code the function that writes the line. A
tool with crumbkit-process's `forward` feature calls
`start_under(forward::clean_up)`. The line goes to
`usage.jsonl` in the tool's directory under the XDG state directory, such
as `~/.local/state/shelf/usage.jsonl`. The guard's type is [`Log`].

A tool records value-free facts of its own, such as the requests it sent
to a server and the time it waited for them, with an [`Extension`] type and
[`extend`]. The extension writes each fact into [`Facts`] under a [`Key`],
and a [`Label`] is one value from the tool's own fixed vocabulary, such
as the route template `/books/{id}`.

A tool adds `self usage` with [`SelfUsage`] and [`findings`],
`self figures list` with [`SelfFigures`] and [`figures`], and
`self runs list` with [`SelfRuns`] and [`runs`]. It passes the result to
crumbkit's `finish` with the document [`SELF_USAGE`],
[`SELF_FIGURES_LIST`] or [`SELF_RUNS_LIST`]. Their rows are
[`FindingRow`], [`FiguresRow`] and [`RunRow`], and [`UsageSummary`] sums
the runs read. shelf's [`cli.rs`] and [`lib.rs`] show all three.

## Where to read more

- [`start`] describes the log, and [`Extension`] the tool's own facts.
- [`SelfUsage`], [`SelfFigures`] and [`SelfRuns`] describe the three
  commands.
- The [schemas] describe the `<tool>/usage/v1`, `<tool>/self-usage/v1`,
  `<tool>/self-figures-list/v1` and `<tool>/self-runs-list/v1` documents.
- The [usage log docs], [trace context docs] and [usage report docs]
  describe the log, its trace context and the three commands, and the
  [run observer docs] describe how crumbkit reports each run to the log.
  [ADR-0029], [ADR-0030], [ADR-0034], [ADR-0095], [ADR-0144],
  [ADR-0145] and [ADR-0190] record why they work this way.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`start`]: src/recorder.rs
[`start_under`]: src/recorder.rs
[`Log`]: src/recorder.rs
[`Facts`]: src/facts.rs
[`Key`]: src/facts.rs
[`Label`]: src/facts.rs
[`SELF_USAGE`]: src/self_usage.rs
[`SELF_FIGURES_LIST`]: src/self_figures.rs
[`FindingRow`]: src/self_usage.rs
[`FiguresRow`]: src/self_figures.rs
[`UsageSummary`]: src/self_usage.rs
[`Extension`]: src/facts.rs
[`extend`]: src/facts.rs
[`SelfUsage`]: src/self_usage.rs
[`findings`]: src/self_usage.rs
[`SelfFigures`]: src/self_figures.rs
[`figures`]: src/self_figures.rs
[`SELF_RUNS_LIST`]: src/self_runs.rs
[`RunRow`]: src/self_runs.rs
[`SelfRuns`]: src/self_runs.rs
[`runs`]: src/self_runs.rs
[`cli.rs`]: ../../examples/shelf/src/cli.rs
[`lib.rs`]: ../../examples/shelf/src/lib.rs
[schemas]: schemas
[ADR-0029]: ../../docs/decisions/api/usage/0029-usage-log.md
[ADR-0030]: ../../docs/decisions/api/usage/0030-trace-context.md
[ADR-0034]: ../../docs/decisions/api/usage/0034-usage-report.md
[ADR-0095]: ../../docs/decisions/api/usage/0095-text-hand-over-own-signals-and-menu.md
[ADR-0144]: ../../docs/decisions/api/usage/0144-a-tool-adds-value-free-facts-to-the-usage-record.md
[ADR-0145]: ../../docs/decisions/api/usage/0145-self-usage-finds-slow-commands-that-mostly-wait.md
[ADR-0190]: ../../docs/decisions/api/usage/0190-self-runs-list-shows-each-sessions-runs-in-order.md
[usage log docs]: ../../docs/behaviour/usage-log/README.md
[trace context docs]: ../../docs/behaviour/usage-log/trace-context.md
[usage report docs]: ../../docs/behaviour/usage-log/report.md
[run observer docs]: ../../docs/behaviour/run-observer/README.md
