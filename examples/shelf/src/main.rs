//! The `shelf` binary, which runs the tool that the shelf library holds.

use std::process::ExitCode;

fn main() -> ExitCode {
    shelf::run()
}
