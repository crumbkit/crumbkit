//! The declared codes that the twin and recovery checks read.

use crumbkit::Code;

/// The declared codes of a check's settings: the `CODES` list of each of
/// the tool's error types, or one list of codes that the tool may build at
/// run time.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Declared<'a> {
    /// The `CODES` list of each of the tool's error types.
    Lists(&'a [&'a [&'a Code]]),
    /// Codes that the test holds by value.
    Codes(&'a [Code]),
}

impl<'a> Declared<'a> {
    /// Returns every declared code in order, with a code that two lists
    /// share once for each list.
    pub(crate) fn to_vec(self) -> Vec<&'a Code> {
        match self {
            Self::Lists(lists) => lists.iter().flat_map(|list| list.iter().copied()).collect(),
            Self::Codes(codes) => codes.iter().collect(),
        }
    }
}
