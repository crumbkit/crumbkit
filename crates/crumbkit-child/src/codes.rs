//! The codes of the failures of a child hand-over (ADR-0042).
//!
//! Each code is in the `child` area and prints with the tool's component,
//! such as `<tool>::child::not_found`, as crumbkit's own codes are. A tool
//! prints its own code in place of any of them with a `Rename` in
//! `Guide::codes`, as it does for crumbkit's codes. [`ALL`] lists them.

use crumbkit::Code;

#[cfg(test)]
mod tests;

/// The code of `--json` passed to a command whose child has no JSON form.
/// It exits with the code of a refused command line unless the guide lists
/// it.
pub const NO_JSON: Code = Code::new("crumbkit", "child", "no_json");

/// The code of a child program that does not exist.
pub const NOT_FOUND: Code = Code::new("crumbkit", "child", "not_found");

/// The code of a child program that the operating system refused to start
/// or to wait for.
pub const NOT_STARTED: Code = Code::new("crumbkit", "child", "not_started");

/// The code of a child's output that is not the declared document under
/// `--json`.
pub const BAD_JSON: Code = Code::new("crumbkit", "child", "bad_json");

/// The code of a child that exited with a code other than 0 under `--json`.
pub const FAILED: Code = Code::new("crumbkit", "child", "failed");

/// Every code above, in the order this module declares them.
pub const ALL: &[&Code] = &[&NO_JSON, &NOT_FOUND, &NOT_STARTED, &BAD_JSON, &FAILED];
