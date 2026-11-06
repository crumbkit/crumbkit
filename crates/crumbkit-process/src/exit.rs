//! How a run ended.

/// How a run's child ended. The caller can tell an exit, a signal, a
/// timeout and a stop apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Exit {
    /// The child exited with this code. On Windows a child in a job that
    /// the crate ended exits with 1.
    Code(i32),
    /// A signal, by its number, ended the child, and the run did not send
    /// it. Only Unix has this exit.
    Signal(i32),
    /// The run's deadline passed before the child exited, and the run
    /// stopped the child.
    TimedOut,
    /// The run stopped the child for the reason given.
    Stopped(Stop),
}

/// Why a run stopped its child before its deadline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Stop {
    /// The run's [`Cancel`](crate::Cancel) was cancelled.
    Cancelled,
    /// An output stream whose [`OverLimit`](crate::OverLimit) is `Stop`
    /// passed its limit, or one of its limits when it is delivered as
    /// lines.
    OverLimit,
}
