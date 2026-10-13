//! The alternatives that a leaf's function returns for a refusal of a
//! command line that names the leaf, kept when they render and their line
//! parses (ADR-0056).

use std::ffi::OsString;

use clap::error::ErrorKind;
use clap::{Command, FromArgMatches};

use super::attempt;
use crate::Call;
use crate::help::{Guide, RefusalContext};

/// Returns the calls that the function of the leaf of `guide` at `path`
/// returns for `error`, with which clap refused `words`, the caller's words
/// after the program name. It keeps each call that renders against
/// `command` and whose line [`parses`], and returns no calls when no leaf
/// is at `path` or the leaf has no function.
///
/// Only a refused line that names a leaf with a function builds the leaf's
/// branch of the tree and parses again, so other refusals cost one lookup
/// in the guide's leaves.
pub(super) fn alternatives<C: FromArgMatches>(
    command: &mut Command,
    guide: &Guide,
    path: &[String],
    error: &clap::Error,
    words: &[OsString],
) -> Vec<Call> {
    let Some(function) = guide
        .get_leaves()
        .iter()
        .find(|leaf| leaf.path() == path)
        .and_then(|leaf| leaf.get_refusal_alternatives())
    else {
        return Vec::new();
    };
    // `RefusalContext::new` displays the leaf's arguments, which clap
    // allows only after a build, and a build of the first command on the
    // path builds the leaf with the global arguments of the root.
    if let Some(first) = path.first()
        && let Some(branch) = command.find_subcommand_mut(first)
    {
        branch.build();
    }
    let mut leaf: &Command = command;
    for name in path {
        let Some(next) = leaf.find_subcommand(name) else {
            return Vec::new();
        };
        leaf = next;
    }
    let calls = function(&RefusalContext::new(leaf, error, words));
    calls
        .into_iter()
        .filter(|call| {
            crate::call::render(call, command).is_ok() && parses::<C>(command, call, guide)
        })
        .collect()
}

/// Reports whether the line of `call`, which already rendered against
/// `command`, parses as `parse` would accept it: clap accepts it or prints
/// help or the version for it, `C` takes its values, and it types no view
/// flag that its leaf does not offer. A call that holds a placeholder has
/// no line to parse and counts as parsing.
fn parses<C: FromArgMatches>(command: &mut Command, call: &Call, guide: &Guide) -> bool {
    let Some(words) = crate::call::words(call, command) else {
        return true;
    };
    match command.try_get_matches_from_mut(words) {
        Ok(mut matches) => {
            attempt::refusal(command, &matches, guide).is_none()
                && C::from_arg_matches_mut(&mut matches).is_ok()
        }
        Err(error) => matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        ),
    }
}
