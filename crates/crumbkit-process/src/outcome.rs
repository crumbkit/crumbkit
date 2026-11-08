//! How a run ended.

#[cfg(test)]
mod tests;

/// How a run's child ended. The caller can tell an exit, a signal, a
/// timeout and a stop apart. [`ChildFinished::status`](crate::ChildFinished::status)
/// gives the child's own status as the run reaped it, which a timeout or a
/// stop also has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outcome {
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

/// How the child process itself ended, as the run reaped it. A run that
/// stopped the child gives the status that the stop caused, such as the
/// signal of its first step or the code that the child's handler of that
/// signal exited with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// The child exited with this code. On Windows a child in a job that
    /// the crate ended exits with 1.
    Code(i32),
    /// A signal, by its number, ended the child. Only Unix has this status.
    Signal(i32),
}

impl Status {
    /// Returns the code that a POSIX shell gives for this status: the exit
    /// code, or 128 plus the signal's number, so a child that SIGINT ended
    /// gives 130.
    #[must_use]
    pub const fn shell_code(self) -> i32 {
        match self {
            Self::Code(code) => code,
            Self::Signal(signal) => 128 + signal,
        }
    }

    /// Returns the exit of a run whose child ended on its own with this
    /// status.
    pub(crate) const fn exit(self) -> Outcome {
        match self {
            Self::Code(code) => Outcome::Code(code),
            Self::Signal(signal) => Outcome::Signal(signal),
        }
    }
}
