//! The exit code as a number, which the usage log and the `tracing` spans
//! record. [`Guide::failure_exit`](crate::help::Guide::failure_exit) gives
//! the exit code of a failed run.

use std::process::ExitCode;

/// Returns `exit`, the exit code of a help page or the version, as a
/// number.
///
/// `ExitCode` cannot be read back as a number, so this compares `exit` with
/// 0 and 2, and any other code counts as 1. A page exits with 0, or with 1
/// when it could not be written, so the number is exact for it. A refusal,
/// `finish` and a stream keep their exit code as a number and never pass it
/// through this function, because the guide can give a failure any code.
pub(crate) fn number(exit: ExitCode) -> u8 {
    if exit == ExitCode::SUCCESS {
        0
    } else if exit == ExitCode::from(2) {
        2
    } else {
        1
    }
}

/// Returns the exit code of a run that has no failure and whose rows report
/// a problem through the field that `Row::PROBLEM` names (ADR-0050).
///
/// This is the one place that chooses that exit code. `finish`, a stream
/// and the note that tells a person about the problem rows all read it.
pub(crate) const fn problem_rows() -> u8 {
    1
}
