// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`hand_over`]: crate::hand_over()
//! [`hand_over_json`]: crate::hand_over_json()
//! [`Json`]: crate::Json
//! [`codes`]: crate::codes
//! [ADR-0042]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/output/0042-child-processes.md
//! [ADR-0057]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0057-run-observer.md
#![doc = include_str!("../README.md")]

pub mod codes;
mod error;
mod hand_over;
mod signals;
#[cfg(test)]
mod testing;

pub use self::hand_over::{Json, hand_over, hand_over_json};
