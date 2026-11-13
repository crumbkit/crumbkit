//! The declared codes that the twin and recovery checks read.

use crumbkit::Code;

/// Returns every code of `lists`, the `CODES` list of each of the tool's
/// error types, in order, with a code that two lists share once for each
/// list.
pub(crate) fn declared<'a>(lists: &[&[Code<'a>]]) -> Vec<Code<'a>> {
    lists.iter().flat_map(|list| list.iter().copied()).collect()
}
