//! The exit code of a run that has no failure and whose rows report a
//! problem. [`Guide::failure_exit`](crate::help::Guide::failure_exit) gives
//! the exit code of a failed run.

/// Returns the exit code of a run that has no failure and whose rows report
/// a problem, as `Row::is_problem` decides (ADR-0050, ADR-0096).
///
/// This is the one place that chooses that exit code. `finish`, a stream
/// and the note that tells a person about the problem rows all read it.
pub(crate) const fn problem_rows() -> u8 {
    1
}
