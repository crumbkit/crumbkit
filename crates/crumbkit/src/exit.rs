//! The exit code as a number, which the usage log and the `tracing` spans
//! record.

use std::process::ExitCode;

/// Returns `exit`, one of crumbkit's own exit codes, as a number.
///
/// `ExitCode` cannot be read back as a number, so this compares `exit` with
/// the three codes that crumbkit owns. crumbkit's `parse` and `finish`
/// return only 0, 1 and 2 (ADR-0002), so the number is exact for them, and
/// any other code would count as 1. A tool's own exit code reaches the
/// usage log as a number through `usage::ended`, and never passes through
/// this function.
pub(crate) fn number(exit: ExitCode) -> u8 {
    if exit == ExitCode::SUCCESS {
        0
    } else if exit == ExitCode::from(2) {
        2
    } else {
        1
    }
}
