// The crate docs are the README, so its examples run as doctests. The README
// links to files in the repository, which work on GitHub. The definition below
// comes first, so rustdoc uses it for the same label (ADR-0013).
//! [ADR-0043]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/effects/0043-pickers-and-interactive-input.md
#![doc = include_str!("../README.md")]

mod menu;

pub use crate::menu::Menu;
