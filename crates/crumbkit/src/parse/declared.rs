//! The alternatives and the hint that a leaf's functions, or else the
//! guide's, return for a refusal of a command line that names the leaf. The
//! alternatives are kept when they render and their line parses (ADR-0056,
//! ADR-0123).

use std::ffi::OsString;

use clap::error::ErrorKind;
use clap::{Command, FromArgMatches};

use super::attempt;
use crate::Call;
use crate::help::{Guide, RefusalContext};

/// What the tool's functions returned for one refused command line.
#[derive(Debug, Default)]
pub(super) struct Declared {
    /// The calls that render and whose line parses, in the function's
    /// order.
    pub(super) calls: Vec<Call>,
    /// The hint, or `None` when no function is set or it returned none.
    pub(super) hint: Option<String>,
}

/// Returns the calls and the hint that the functions of the leaf of `guide`
/// at `path`, or else the guide's functions, return for `error`, with which
/// clap refused `words`, the caller's words after the program name. It
/// keeps each call that renders against `command` and whose line
/// [`parses`], and returns no calls and no hint when no function is set.
///
/// Only a refused line with a function builds the branch of the tree on
/// `path` and parses again, so for a guide without a function of its own
/// other refusals cost two lookups in the guide's leaves.
pub(super) fn declared<C: FromArgMatches>(
    command: &mut Command,
    guide: &Guide,
    path: &[String],
    error: &clap::Error,
    words: &[OsString],
) -> Declared {
    let alternatives = guide.refusal_function(path);
    let hint = guide.refusal_hint_function(path);
    if alternatives.is_none() && hint.is_none() {
        return Declared::default();
    }
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
            return Declared::default();
        };
        leaf = next;
    }
    let context = RefusalContext::new(leaf, error, words);
    let calls = alternatives.map_or_else(Vec::new, |function| function(&context));
    let hint = hint.and_then(|function| function(&context));
    let calls = calls
        .into_iter()
        .filter(|call| {
            crate::call::render(call, command).is_ok() && parses::<C>(command, call, guide)
        })
        .collect();
    Declared { calls, hint }
}

/// Reports whether the line of `call`, which already rendered against
/// `command`, parses as `parse` would accept it: clap accepts it or prints
/// help or the version for it, `C` takes its values, and it types no view
/// flag that its leaf does not offer and no `--fields` name that its
/// leaf's document lacks. A call that holds a placeholder has
/// no line to parse and counts as parsing.
fn parses<C: FromArgMatches>(command: &mut Command, call: &Call, guide: &Guide) -> bool {
    let Some(words) = crate::call::words(call, command) else {
        return true;
    };
    match command.try_get_matches_from_mut(words) {
        Ok(mut matches) => {
            let (path, node, level) = attempt::deepest(command, &matches);
            attempt::refusal(node, level, &path, guide).is_ok()
                && C::from_arg_matches_mut(&mut matches).is_ok()
        }
        Err(error) => matches!(
            error.kind(),
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
        ),
    }
}
