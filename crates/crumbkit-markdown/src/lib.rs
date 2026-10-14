// The crate docs are the README, so its examples run as doctests. The README
// links to files in the repository, which work on GitHub. The definitions
// below come first, so rustdoc uses them for the same labels (ADR-0013).
//! [ADR-0044]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0044-command-reference.md
//! [`render`]: crate::render
//! [`Reference`]: crate::Reference
//! [`RenderError`]: crate::RenderError
//! [command reference docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/help/reference.md
#![doc = include_str!("../README.md")]

mod document;
mod error;
mod render;
mod text;

pub use crate::error::RenderError;
pub use crate::render::{Reference, render};
