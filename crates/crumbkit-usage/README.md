# crumbkit-usage

crumbkit-usage keeps a local usage log for a tool built with crumbkit. The
log is one JSON line per run in a file on the user's machine. The line
records the shape of the run, such as the command path, the flags typed and
the exit code, and never the values that the caller typed. The tool's
`self usage` and `self figures list` commands read the log back. crumbkit is
in design and is not published yet.

## Adding it

A tool adds crumbkit-usage beside crumbkit, from this repository until the
first release:

```toml
[dependencies]
crumbkit-usage = { git = "https://github.com/crumbkit/crumbkit" }
```

The `main` branch can change without notice before the first release. The
crate builds on Unix and Windows.

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
observer keeps it, and both observers hear each report. The line
goes to `usage.jsonl` in the tool's directory under the XDG state directory,
such as `~/.local/state/shelf/usage.jsonl`.

A tool adds `self usage` with [`SelfUsage`] and [`findings`], and
`self figures list` with [`SelfFigures`] and [`figures`]. shelf's
[`cli.rs`] and [`lib.rs`] show both.

## Where to read more

- [`start`], [`start_with`], [`Signals`] and [`section`] describe the log.
- [`SelfUsage`] and [`SelfFigures`] describe the two commands.
- The [schemas] describe the `crumbkit/usage/v1`, `crumbkit/self-usage/v1`
  and `crumbkit/self-figures-list/v1` documents.
- The [usage log docs], [trace context docs] and [usage report docs]
  describe the log, its trace context and the two commands, and the
  [run observer docs] describe how crumbkit reports each run to the log.
  [ADR-0029], [ADR-0030] and [ADR-0034] record why they work this way.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`start`]: src/recorder.rs
[`start_with`]: src/recorder.rs
[`Signals`]: src/recorder.rs
[`section`]: src/recorder.rs
[`SelfUsage`]: src/self_usage.rs
[`findings`]: src/self_usage.rs
[`SelfFigures`]: src/self_figures.rs
[`figures`]: src/self_figures.rs
[`cli.rs`]: ../../examples/shelf/src/cli.rs
[`lib.rs`]: ../../examples/shelf/src/lib.rs
[schemas]: schemas
[ADR-0029]: ../../docs/decisions/api/usage/0029-usage-log.md
[ADR-0030]: ../../docs/decisions/api/usage/0030-trace-context.md
[ADR-0034]: ../../docs/decisions/api/usage/0034-usage-report.md
[usage log docs]: ../../docs/behaviour/usage-log/README.md
[trace context docs]: ../../docs/behaviour/usage-log/trace-context.md
[usage report docs]: ../../docs/behaviour/usage-log/report.md
[run observer docs]: ../../docs/behaviour/run-observer/README.md
