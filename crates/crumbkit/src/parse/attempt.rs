//! One parse of the command line: the tool's parsed type, `--no-input`,
//! `--dry-run`, and the view that the view flags select, or the refusal of a view flag
//! that the typed command does not offer, of two view flags that cannot
//! work together, or of a `--fields` name that its document lacks.

use std::ffi::OsString;

use clap::parser::ValueSource;
use clap::{ArgMatches, Command, FromArgMatches};

use super::pick;
use crate::ViewFlags;
use crate::help::{Guide, Leaf, ViewFlag, view};
use crate::output::{dry_run, fields};
use crate::refusal::Refusal;
use crate::view::View;

/// What one parse of the command line found.
pub(super) struct Attempt<C> {
    /// The tool's parsed type.
    pub(super) parsed: C,
    /// The view that the typed view flags select, or the refusal of a view
    /// flag that [`refusal`] finds. The refusal sits in a `Box`, so the
    /// `Result` stays small on the path that parses.
    pub(super) view: Result<View, Box<Refusal>>,
    /// Whether the caller passed `--no-input`.
    pub(super) no_input: bool,
    /// Whether the caller passed crumbkit's `--dry-run` on the deepest
    /// command.
    pub(super) dry_run: bool,
}

/// Parses `args` against `command`, and returns the tool's parsed type,
/// whether the caller passed `--no-input` and `--dry-run`, which parsing
/// reads before the tool's type takes the values out of the matches, and
/// the view that the view flags select or the refusal of a view flag that
/// [`refusal`] finds.
pub(super) fn attempt<C: FromArgMatches>(
    command: &mut Command,
    args: Vec<OsString>,
    guide: &Guide,
) -> Result<Attempt<C>, clap::Error> {
    let mut matches = command.try_get_matches_from_mut(args)?;
    crate::events::parsed(command, &matches);
    let no_input = pick::no_input(&matches);
    let (path, node, level) = deepest(command, &matches);
    let dry_run = dry_run::typed(level);
    let view = refusal(node, level, &path, guide);
    C::from_arg_matches_mut(&mut matches).map(|parsed| Attempt {
        parsed,
        view,
        no_input,
        dry_run,
    })
}

/// Returns the path of the deepest command in `matches`, that command in
/// `command`, and its matches.
pub(super) fn deepest<'a>(
    command: &'a Command,
    matches: &'a ArgMatches,
) -> (Vec<&'a str>, &'a Command, &'a ArgMatches) {
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
    (path, node, level)
}

/// Returns the view that the typed view flags select, or the refusal of
/// the view flags that the caller typed.
///
/// It first refuses the first view flag, in the order `--fields`, `--long`,
/// `--full`, that the leaf of `guide` at `path` does not offer (ADR-0016).
/// `path` names `node`, the deepest command that the caller typed, and
/// `level` holds its matches. On a leaf whose document is headed, it then
/// refuses `--fields` without `--json`, as [`view::headed`] describes
/// (ADR-0062). On a leaf whose row type sets `Row::WHOLE`, it then refuses
/// `--long` with `--json`, as [`view::whole_long`] describes (ADR-0138).
/// It then refuses two typed flags that cannot work together,
/// on every command, as [`View::try_from`] describes (ADR-0093). On a leaf
/// with a document, it last refuses a `--fields` name that the row type of
/// the document lacks, as [`unknown_field`] describes.
///
/// clap copies the value of a global flag to every command on the path, so
/// the deepest command's matches hold a view flag that the caller typed
/// before the command path too. A flag counts only when the command's
/// argument with the flag's id is global, so a tool's own argument with
/// that id is never refused.
pub(super) fn refusal(
    node: &Command,
    level: &ArgMatches,
    path: &[&str],
    guide: &Guide,
) -> Result<View, Box<Refusal>> {
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
    let leaf = guide.get_leaves().iter().find(|leaf| leaf.path() == path);
    let unoffered = leaf.and_then(|leaf| {
        ViewFlag::ALL.into_iter().find_map(|flag| {
            let reason = leaf.unoffered(flag)?;
            typed(flag.id()).then(|| view::refusal(flag, reason))
        })
    });
    if let Some(refusal) = unoffered {
        return Err(Box::new(refusal));
    }
    let headed = leaf
        .and_then(Leaf::get_document)
        .is_some_and(|document| document.is_headed());
    if headed && let Some(refusal) = view::headed(typed(ViewFlag::Fields.id()), typed("json")) {
        return Err(Box::new(refusal));
    }
    let whole = leaf.is_some_and(Leaf::rows_whole);
    if let Some(refusal) = view::whole_long(whole, typed(ViewFlag::Long.id()), typed("json")) {
        return Err(Box::new(refusal));
    }
    let view = View::try_from(ViewFlags::read(node, level)).map_err(Box::new)?;
    if let Some(leaf) = leaf
        && let Some(refusal) = unknown_field(&view, leaf)
    {
        return Err(Box::new(refusal));
    }
    Ok(view)
}

/// Returns the refusal of the first `--fields` name in `view` that is not
/// a field of the row type of `leaf`'s document, or `None` when every name
/// is a field or the leaf has no document. The refusal is the one that
/// `finish` prints for an unknown name in a result, so a mistyped name
/// fails before the handler runs, also on a run whose handler would fail.
fn unknown_field(view: &View, leaf: &Leaf) -> Option<Refusal> {
    // The names are read first, so a leaf's row schema is built only when
    // the caller passed `--fields`.
    let names = view.names()?;
    fields::unknown_field_in(&leaf.row_schema()?, names)
}
