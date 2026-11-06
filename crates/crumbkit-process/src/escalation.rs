//! The steps that stop a child: signals, each with a grace period, and then
//! a kill.

use std::time::Duration;

use crate::Signal;

#[cfg(test)]
mod tests;

/// How a run stops its child: each step sends a signal to the group and
/// waits its grace for the child to exit, and a kill follows the last step.
///
/// The default escalation has no steps, so a stop kills at once. Each caller
/// states its own steps, because a graceful signal helps only a child that
/// handles it:
///
/// ```
/// use std::time::Duration;
///
/// use crumbkit_process::{Escalation, Signal};
///
/// let polite = Escalation::default()
///     .step(Signal::Interrupt, Duration::from_secs(2))
///     .step(Signal::Terminate, Duration::from_secs(5));
/// assert_ne!(polite, Escalation::default());
/// ```
///
/// On Windows a stop skips the steps and ends the job at once (ADR-0171).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Escalation {
    steps: Vec<(Signal, Duration)>,
}

impl Escalation {
    /// Returns this escalation with one more step, which sends `signal` and
    /// waits at most `grace` for the child to exit.
    #[must_use]
    pub fn step(mut self, signal: Signal, grace: Duration) -> Self {
        self.steps.push((signal, grace));
        self
    }

    /// Returns the steps in order, each as its signal and its grace. The
    /// kill that follows them is not a step.
    pub(crate) fn steps(&self) -> impl Iterator<Item = (Signal, Duration)> + '_ {
        self.steps.iter().copied()
    }
}
