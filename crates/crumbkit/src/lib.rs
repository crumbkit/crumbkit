// The crate docs are the README, so its example runs as a doctest.
#![doc = include_str!("../README.md")]

mod document;
mod output;
mod row;
mod segment;
mod sink;

pub use crate::document::Document;
pub use crate::output::{Output, finish, finish_to};
pub use crate::row::{Row, Rows};
pub use crate::sink::Sink;
