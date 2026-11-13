//! The codes of the failures of a child hand-over (ADR-0042).
//!
//! Each code is in the `child` area and prints with the tool's component,
//! such as `<tool>::child::not_found`, as crumbkit's own codes are. [`ALL`]
//! lists them.

use crumbkit::Code;

#[cfg(test)]
mod tests;

/// The code of a child program that does not exist.
pub const NOT_FOUND: Code<'static> = Code::new("crumbkit", "child", "not_found");

/// The code of a child program that the operating system refused to start
/// or to wait for.
pub const NOT_STARTED: Code<'static> = Code::new("crumbkit", "child", "not_started");

/// Both codes above, in the order this module declares them.
pub const ALL: &[Code<'static>] = &[NOT_FOUND, NOT_STARTED];
