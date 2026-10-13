//! The exit code as a number, which the usage log and the `tracing` spans
//! record, and the exit code of a failed run.

use std::process::ExitCode;

use crate::Code;
use crate::help::Exit;

/// Returns `exit`, one of crumbkit's own exit codes, as a number.
///
/// `ExitCode` cannot be read back as a number, so this compares `exit` with
/// the three codes that crumbkit owns. `parse` returns only 0 and 2 when it
/// ends a run, so the number is exact for it, and any other code would
/// count as 1. `finish` and a stream keep their exit code as a number and
/// never pass it through this function, and a tool's own exit code reaches
/// the usage log as a number through `usage::ended`.
pub(crate) fn number(exit: ExitCode) -> u8 {
    if exit == ExitCode::SUCCESS {
        0
    } else if exit == ExitCode::from(2) {
        2
    } else {
        1
    }
}

/// Returns the exit code of a run whose first failure has `code`: the code
/// of the first of `exits` whose failures list `code`, or 1 when none does
/// (ADR-0037).
pub(crate) fn failed(exits: &[Exit], code: &Code) -> u8 {
    exits
        .iter()
        .find(|exit| exit.get_failures().contains(&code))
        .map_or(1, Exit::code)
}
