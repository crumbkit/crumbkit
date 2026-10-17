//! The codes of the failures of the aliases file (ADR-0027).
//!
//! Each code is in the `aliases` area and prints with the tool's component,
//! such as `<tool>::aliases::unreadable`, as crumbkit's own codes are
//! (ADR-0079). A tool prints its own code in place of any of them with a
//! `Rename` in `Guide::codes`. [`ALL`] lists them.

use crumbkit::Code;

/// The code of an aliases file that exists but cannot be read or parsed.
pub const UNREADABLE: Code = Code::new("crumbkit", "aliases", "unreadable");

/// Every code above, in the order this module declares them.
pub const ALL: &[&Code] = &[&UNREADABLE];
