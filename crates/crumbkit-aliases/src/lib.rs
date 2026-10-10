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
