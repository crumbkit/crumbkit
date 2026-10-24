//! A value type in a crate whose only dependency is crumbkit, which proves
//! that `crumbkit::shown!` needs no serde or schemars in the calling crate.
//! The workspace build compiles it on every pull request.

use std::fmt;

use crumbkit::Shown;

#[cfg(test)]
mod tests;

/// A shelf's state, which reads `open` or `closed` in text and in JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The shelf takes loans.
    Open,
    /// The shelf takes no loans.
    Closed,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Open => "open",
            Self::Closed => "closed",
        })
    }
}

impl Shown for State {}

crumbkit::shown!(State);
