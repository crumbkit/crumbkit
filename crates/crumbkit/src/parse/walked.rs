//! The reading of the words that crumbkit gave clap, which `Refused` and
//! `Stop` share for their `flags` and `value` getters.

use std::ffi::{OsStr, OsString};

use clap::Command;

use super::walk::given;

/// Returns the clap ids of the arguments that `words`, the words after the
/// program name, pass as flags to `command`, in the order typed, as
/// [`given`] reads them.
pub(super) fn flags(command: &Command, words: &[OsString]) -> Vec<String> {
    given(command, words, false).ids
}

/// Returns the last value that `words`, the words after the program name,
/// give the option of `command` whose clap id is `id`, as [`given`] reads
/// them, or `None` when they give it none.
pub(super) fn value<'w>(command: &Command, words: &'w [OsString], id: &str) -> Option<&'w OsStr> {
    given(command, words, false)
        .values
        .iter()
        .rev()
        .find(|(arg, _)| arg.get_id().as_str() == id)
        .map(|&(_, value)| value)
}
