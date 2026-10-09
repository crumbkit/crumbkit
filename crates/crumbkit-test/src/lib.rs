// The crate docs are the README, so its examples run as doctests.
#![doc = include_str!("../README.md")]

mod codes;
mod filters;
mod grammar;
mod guide;
mod listings;
mod names;
#[cfg(test)]
mod testing;
mod tree;

pub use crate::codes::check_codes;
pub use crate::filters::check_filters;
pub use crate::grammar::check_grammar;
pub use crate::guide::check_guide;
pub use crate::listings::check_listings;
pub use crate::names::check_names;
