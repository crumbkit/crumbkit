// The crate docs are the README, so its example runs as a doctest.
#![doc = include_str!("../README.md")]

mod call;
mod code;
mod document;
mod failure;
mod listing;
mod output;
mod page;
#[cfg(test)]
mod property;
mod row;
mod segment;
mod sink;

pub use crate::call::{Call, CallError};
pub use crate::code::{Code, Spelling};
pub use crate::document::Document;
pub use crate::failure::{AsFailure, Error, Failure, Recovery};
pub use crate::listing::Listing;
pub use crate::output::{Output, finish, finish_to};
pub use crate::page::Page;
pub use crate::row::{Row, Rows};
pub use crate::sink::Sink;
