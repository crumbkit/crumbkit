// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`file`]: crate::file()
//! [`expand`]: crate::expand()
//! [`list`]: crate::list()
//! [`AliasesList`]: crate::AliasesList
//! [`ALIASES`]: crate::ALIASES
//! [schema]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-aliases/schemas/aliases-v1.json
//! [ADR-0027]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0027-aliases.md
#![doc = include_str!("../README.md")]

mod entry;
mod expand;
mod file;
mod list;
#[cfg(test)]
mod testing;

pub use crate::expand::expand;
pub use crate::file::file;
pub use crate::list::{ALIASES, AliasRow, AliasesList, list};
