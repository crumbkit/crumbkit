// The crate docs are the README, so its example runs as a doctest.
#![doc = include_str!("../README.md")]

mod batch;
mod call;
mod code;
mod document;
mod failure;
mod fetch;
pub mod help;
mod listing;
mod output;
mod page;
mod parse;
mod refusal;
#[cfg(feature = "miette")]
mod report;
mod row;
mod segment;
mod sink;

pub use crate::batch::Batch;
pub use crate::call::{Call, CallError};
pub use crate::code::{Code, Spelling};
pub use crate::document::Document;
pub use crate::failure::{AsFailure, Error, Failure, Recovery};
pub use crate::fetch::fetch_each;
pub use crate::listing::Listing;
pub use crate::output::{Output, finish, finish_to};
pub use crate::page::Page;
pub use crate::parse::{parse, parse_from, parse_to};
pub use crate::row::{Row, Rows};
pub use crate::sink::Sink;
