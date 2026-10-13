//! One parse of the command line: the tool's parsed type, `--no-input`,
//! and the refusal of a view flag that the typed command does not offer.

use std::ffi::OsString;

use clap::parser::ValueSource;
use clap::{ArgMatches, Command, FromArgMatches};

use super::pick;
use crate::help::{Guide, ViewFlag, view};
use crate::refusal::Refusal;

/// Parses `args` against `command`, and returns the tool's parsed type,
/// whether the caller passed `--no-input`, which parsing reads before the
/// tool's type takes the values out of the matches, and the refusal of a
/// view flag that [`refusal`] finds.
pub(super) fn attempt<C: FromArgMatches>(
    command: &mut Command,
    args: Vec<OsString>,
    guide: &Guide,
) -> Result<(C, bool, Option<Refusal>), clap::Error> {
    let mut matches = command.try_get_matches_from_mut(args)?;
    #[cfg(feature = "usage-log")]
    crate::usage::parsed(command, &matches);
    let no_input = pick::no_input(&matches);
    let refusal = refusal(command, &matches, guide);
    C::from_arg_matches_mut(&mut matches).map(|parsed| (parsed, no_input, refusal))
}

/// Returns the refusal of the view flags that the caller typed, or `None`
/// when the typed command accepts them.
///
/// It first refuses the first view flag, in the order `--fields`, `--long`,
/// `--full`, that the leaf of `guide` for the deepest command in `matches`
/// does not offer (ADR-0016). It then refuses two typed flags that cannot
/// work together, on every command, as [`view::pairing`] describes.
///
/// clap copies the value of a global flag to every command on the path, so
/// the deepest command's matches hold a view flag that the caller typed
/// before the command path too. A flag counts only when the command's
/// argument with the flag's id is global, so a tool's own argument with
/// that id is never refused.
pub(super) fn refusal(command: &Command, matches: &ArgMatches, guide: &Guide) -> Option<Refusal> {
    let mut path: Vec<&str> = Vec::new();
    let mut node = command;
    let mut level = matches;
    while let Some((name, next)) = level.subcommand()
        && let Some(subcommand) = node.find_subcommand(name)
    {
        path.push(name);
        node = subcommand;
        level = next;
    }
    let typed = |id: &str| {
        let global = node
            .get_arguments()
            .any(|arg| arg.get_id() == id && arg.is_global_set());
        // `value_source` needs an id that the matches know, so the id is
        // looked up among them first.
        global
            && level.ids().any(|known| known == id)
            && level.value_source(id) == Some(ValueSource::CommandLine)
    };
    let leaf = guide
        .get_leaves()
        .iter()
        .find(|leaf| leaf.path() == path.as_slice());
    let unoffered = leaf.and_then(|leaf| {
        ViewFlag::ALL.into_iter().find_map(|flag| {
            let reason = leaf.unoffered(flag)?;
            typed(flag.id()).then(|| view::refusal(flag, reason))
        })
    });
    unoffered.or_else(|| {
        view::pairing(view::Typed {
            json: typed("json"),
            fields: typed(ViewFlag::Fields.id()),
            long: typed(ViewFlag::Long.id()),
            full: typed(ViewFlag::Full.id()),
        })
    })
}
