// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`write_atomically`]: crate::write_atomically
//! [`Mode`]: crate::Mode
//! [`WriteError`]: crate::WriteError
//! [ADR-0221]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/effects/0221-the-durable-crate.md
//! [durable files docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/durable-files.md
#![doc = include_str!("../README.md")]

mod platform;
mod write;

pub use crate::write::{Mode, WriteError, WriteStep, write_atomically};
