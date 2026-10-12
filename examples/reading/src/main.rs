//! The `reading` binary, which runs the tool that the reading library holds.

use std::process::ExitCode;

fn main() -> ExitCode {
    reading::run()
}
