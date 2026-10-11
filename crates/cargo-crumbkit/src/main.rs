//! The `cargo-crumbkit` binary, which runs the tool that the cargo-crumbkit
//! library holds. cargo runs it for `cargo crumbkit`.

use std::process::ExitCode;

fn main() -> ExitCode {
    cargo_crumbkit::run()
}
