// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`run`]: crate::run()
//! [`RunOptions`]: crate::RunOptions
//! [`Escalation`]: crate::Escalation
//! [`line_queue`]: crate::line_queue
//! [`forward`]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-process/src/forward.rs
//! [`Outcome`]: crate::Outcome
//! [ADR-0171]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/processes/0171-the-process-crate.md
//! [ADR-0172]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/processes/0172-process-crate-additions.md
//! [ADR-0174]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/processes/0174-stopped-run-keeps-child-status.md
//! [processes docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/processes.md
#![doc = include_str!("../README.md")]

mod cancel;
mod escalation;
#[cfg(all(unix, feature = "forward"))]
pub mod forward;
mod group;
mod lines;
mod notice;
mod outcome;
mod run;
mod sys;
#[cfg(test)]
mod testing;

pub use self::cancel::Cancel;
pub use self::escalation::Escalation;
pub(crate) use self::group::Group;
pub use self::group::{Scope, Signal};
pub use self::lines::{LineQueue, Lines, Silent, line_queue};
pub use self::outcome::{Outcome, Status, StopReason};
pub use self::run::{
    Capture, Captured, ChildFinished, DRAIN_GRACE, Input, OverLimit, RunError, RunOptions, run,
};
