//! The declared codes that the twin and recovery checks read.

use crumbkit::Code;

/// The codes that a check reads: one list, or one list for each of the
/// tool's error types.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CodeLists<'a> {
    /// One flat list, such as a derived `CODES`.
    One(&'a [Code<'a>]),
    /// The `CODES` list of each error type.
    Many(&'a [&'a [Code<'a>]]),
}

/// Returns every code of `lists`, in order, with a code that two lists share
/// once.
pub(crate) fn declared(lists: CodeLists<'_>) -> Vec<Code<'_>> {
    let mut declared: Vec<Code<'_>> = Vec::new();
    let all: Vec<Code<'_>> = match lists {
        CodeLists::One(list) => list.to_vec(),
        CodeLists::Many(lists) => lists.iter().flat_map(|list| list.iter().copied()).collect(),
    };
    for code in all {
        if !declared.contains(&code) {
            declared.push(code);
        }
    }
    declared
}
