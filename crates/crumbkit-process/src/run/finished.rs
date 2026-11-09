//! What a run gives back: how its child ended and what the run kept.

use std::fmt;
use std::io;
use std::time::Duration;

use super::Captured;
use crate::{Outcome, Status};

/// How a run's child ended and what the run kept of its output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildFinished {
    pub(super) outcome: Outcome,
    pub(super) status: Option<Status>,
    pub(super) stdout: Captured,
    pub(super) stderr: Captured,
    pub(super) first_stdout: Option<Duration>,
}

impl ChildFinished {
    /// Returns how the child ended.
    #[must_use]
    pub const fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// Returns the child's own status as the run reaped it, or `None` when
    /// the run started no child because its cancel was already cancelled.
    ///
    /// The status tells how the child ended even when the run stopped it.
    /// A run whose cancel stopped a child that SIGINT ended gives
    /// `Outcome::Stopped(StopReason::Cancelled)` and the status `Status::Signal(2)`,
    /// whose [`Status::shell_code`] is 130. A child that handles the signal
    /// and exits with a code of its own gives that code.
    #[must_use]
    pub const fn status(&self) -> Option<Status> {
        self.status
    }

    /// Returns what the run kept of standard output.
    #[must_use]
    pub const fn stdout(&self) -> &Captured {
        &self.stdout
    }

    /// Returns what the run kept of standard error.
    #[must_use]
    pub const fn stderr(&self) -> &Captured {
        &self.stderr
    }

    /// Returns what the run kept of standard output and standard error.
    #[must_use]
    pub fn into_output(self) -> (Captured, Captured) {
        (self.stdout, self.stderr)
    }

    /// Returns the time from the child's start to the first byte of its
    /// standard output, or `None` when it printed none or the run neither
    /// kept nor copied standard output.
    #[must_use]
    pub const fn first_stdout(&self) -> Option<Duration> {
        self.first_stdout
    }
}

/// A run that could not start its child or follow it to its end.
#[derive(Debug)]
#[non_exhaustive]
pub enum RunError {
    /// The program does not exist or cannot be started, or the hook of
    /// [`RunOptions::around_spawn`](crate::RunOptions::around_spawn) returned
    /// this error, or an [`Input::File`](crate::Input::File) could not be
    /// duplicated. No child is left running.
    Start(io::Error),
    /// Waiting for the child, reaping it, reading its output or starting a
    /// thread failed after it started, or a copy function panicked. The
    /// child was stopped and reaped, or after a panic it ran to its end and
    /// was reaped.
    Run(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Start(_) => "cannot start the program",
            Self::Run(_) => "cannot follow the program to its end",
        })
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Start(error) | Self::Run(error) => Some(error),
        }
    }
}
