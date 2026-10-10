//! The `shelf` binary, which runs the tool that the shelf library holds.

use std::process::ExitCode;

use clap::CommandFactory as _;

fn main() -> ExitCode {
    // The guard writes this run's line to shelf's usage log when `main`
    // returns, after shelf's output is flushed (ADR-0029).
    let _log = crumbkit::usage::start(&shelf::Cli::command());
    shelf::run()
}
