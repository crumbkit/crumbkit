// The crate docs are the README, so its example runs as a doctest.
#![doc = include_str!("../README.md")]

mod codes;

pub use crate::codes::check_codes;
