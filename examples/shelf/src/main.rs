//! The `shelf` binary, which runs the tool that the shelf library holds.

use std::process::ExitCode;

use clap::CommandFactory as _;

fn main() -> ExitCode {
    // A shell's completion request ends the run here, before the usage log
    // starts, so a Tab press writes no line (ADR-0048). shelf parses against
    // `Cli::command()`, so completion offers what parsing accepts.
    if let Some(code) = crumbkit::complete(shelf::Cli::command) {
        return code;
    }
    // The guard writes this run's line to shelf's usage log when `main`
    // returns, after shelf's output is flushed (ADR-0029).
    let _log = crumbkit::usage::start();
    shelf::run()
}
