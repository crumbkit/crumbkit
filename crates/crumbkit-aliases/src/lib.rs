// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`file`]: crate::file()
//! [`Aliases`]: crate::Aliases
//! [`Aliases::expand`]: crate::Aliases::expand
//! [`Aliases::list`]: crate::Aliases::list
//! [`Source`]: crate::Source
//! [`Source::file`]: crate::Source::file
//! [`Source::table`]: crate::Source::table
//! [`Source::lazy`]: crate::Source::lazy
//! [`Kind`]: crate::Kind
//! [`Entry`]: crate::Entry
//! [`AliasesList`]: crate::AliasesList
//! [`ALIASES_LIST`]: crate::ALIASES_LIST
//! [`AliasRow`]: crate::AliasRow
//! [schema]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit-aliases/schemas/aliases-list-v1.json
//! [ADR-0027]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0027-aliases.md
//! [ADR-0228]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/aliases/0228-aliases-come-from-ordered-sources.md
//! [aliases docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/parsing/aliases.md
#![doc = include_str!("../README.md")]

mod aliases;
mod entry;
mod expand;
mod file;
mod judge;
mod list;
mod names;
mod source;
#[cfg(test)]
mod testing;

pub use crate::aliases::Aliases;
pub use crate::file::file;
pub use crate::list::{ALIASES_LIST, AliasRow, AliasesList};
pub use crate::source::{Entry, Kind, Source, Table};
