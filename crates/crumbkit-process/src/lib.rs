// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`run`]: crate::run()
//! [`Run`]: crate::Run
//! [`Group`]: crate::Group
//! [`Escalation`]: crate::Escalation
//! [`Exit`]: crate::Exit
//! [ADR-0171]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/processes/0171-the-process-crate.md
//! [processes docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/processes.md
#![doc = include_str!("../README.md")]

mod cancel;
mod escalation;
mod exit;
mod group;
mod notice;
mod run;
mod sys;
#[cfg(test)]
mod testing;

pub use self::cancel::Cancel;
pub use self::escalation::Escalation;
pub use self::exit::{Exit, Stop};
pub use self::group::{Group, Scope, Signal};
pub use self::run::{
    Capture, Captured, DRAIN_GRACE, Finished, Input, OverLimit, Run, RunError, run,
};
