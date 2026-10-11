//! The exit code as a number, which the usage log and the `tracing` spans
//! record.

use std::process::ExitCode;

/// Returns `exit` as a number. crumbkit's `parse` and `finish` return only
/// 0, 1 and 2 (ADR-0002), and `ExitCode` cannot be read back as a number, so
/// any other code counts as 1.
pub(crate) fn number(exit: ExitCode) -> u8 {
    if exit == ExitCode::SUCCESS {
        0
    } else if exit == ExitCode::from(2) {
        2
    } else {
        1
    }
}
