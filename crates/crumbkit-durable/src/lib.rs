// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`write_atomically`]: crate::write_atomically
//! [`Mode`]: crate::Mode
//! [`WriteError`]: crate::WriteError
//! [`PathLock::acquire`]: crate::PathLock::acquire
//! [`PathLock::still_current`]: crate::PathLock::still_current
//! [`LockError`]: crate::LockError
//! [ADR-0221]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/effects/0221-the-durable-crate.md
//! [ADR-0222]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/effects/0222-a-path-lock-checks-its-file-is-still-at-the-path.md
//! [durable files docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/durable-files.md
#![doc = include_str!("../README.md")]

mod lock;
mod platform;
mod write;

pub use crate::lock::{LockError, LockStep, PathLock};
pub use crate::write::{Mode, WriteError, WriteStep, write_atomically};
