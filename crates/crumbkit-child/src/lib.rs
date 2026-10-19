// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`hand_over`]: crate::hand_over()
//! [`codes`]: crate::codes
//! [ADR-0042]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/output/0042-child-processes.md
//! [ADR-0058]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/output/0058-child-crate.md
//! [ADR-0095]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/usage/0095-text-hand-over-own-signals-and-menu.md
//! [child processes docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/output/child-processes.md
//! [run observer docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/run-observer/README.md
#![doc = include_str!("../README.md")]

pub mod codes;
mod error;
mod hand_over;
#[cfg(doctest)]
mod removed;
mod signals;
#[cfg(test)]
mod testing;

pub use self::hand_over::hand_over;
