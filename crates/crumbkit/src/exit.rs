//! The exit code of a run that has no failure and whose rows report a
//! problem. [`Tool::failure_exit`](crate::Tool::failure_exit) gives
//! the exit code of a failed run.

/// Returns the exit code of a run that has no failure and whose rows report
/// a problem, as `Row::PROBLEM` decides (ADR-0050, ADR-0096).
///
/// This is the one place that chooses that exit code. `finish`, a stream
/// and the note that tells a person about the problem rows all read it.
pub(crate) const fn problem_rows() -> u8 {
    1
}
