//! `--dry-run`, which asks a command to report what it would change and
//! change nothing.

use std::borrow::Cow;

use clap::{ArgMatches, Args};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use super::Output;

#[cfg(test)]
mod tests;

/// The help text of `--dry-run`, which the schema also uses as its
/// description.
const DRY_RUN_HELP: &str = "Report what the command would change, and change nothing";

/// The line on standard error that marks a dry run's result.
pub(super) const DRY_RUN_LINE: &str = "This was a dry run, so nothing changed.";

/// The clap id of `--dry-run`, which [`parse`](crate::parse()) reads from
/// the matches of the deepest command, and which crumbkit-test's
/// `check_effects` tells apart from a tool's own `--dry-run`.
pub(crate) const ID: &str = "crumbkit_dry_run";

/// The flag `--dry-run`, which asks a command to report what it would change
/// and change nothing.
///
/// A command that changes something adds it to its own arguments struct
/// with `#[command(flatten)]`. [`parse`](crate::parse()) reads the flag by
/// its id from the matches of the deepest command, before the tool's type
/// takes its values, and records it in the [`Output`] it returns, so the
/// handler reads [`Output::is_dry_run`] and the tool's own copy of the flag
/// is unused. A tool that parses with clap alone passes the parsed flag to
/// [`Output::new`]:
///
/// ```
/// use clap::{CommandFactory as _, Parser};
/// use crumbkit::help::Guide;
/// use crumbkit::{DryRun, Output, ViewFlags};
///
/// /// Remove books from the catalogue.
/// #[derive(Parser)]
/// struct BooksRemove {
///     #[command(flatten)]
///     dry_run: DryRun,
///     #[command(flatten)]
///     view: ViewFlags,
/// }
///
/// let words = ["books-remove", "--dry-run"];
/// let args = BooksRemove::parse_from(words);
/// let output = Output::new(args.view, args.dry_run, BooksRemove::command(), &Guide::new(), words);
/// assert!(output.unwrap().is_dry_run());
/// ```
///
/// The flag's clap id is `crumbkit_dry_run`, so crumbkit-test's
/// `check_effects` tells it apart from a tool's own `--dry-run`. It has no
/// getter, so a tool reads it only through `Output::is_dry_run`, and
/// `finish` always knows when a run is a dry run.
#[derive(Args, Clone, Copy, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
// clap builds the arguments of every command on every run, so the type
// skips the argument group that clap would build for its one flag.
#[group(skip)]
pub struct DryRun {
    #[arg(id = ID, long = "dry-run", help = DRY_RUN_HELP)]
    dry_run: bool,
}

impl DryRun {
    /// Reports whether the flag is on.
    pub(crate) const fn is_on(self) -> bool {
        self.dry_run
    }
}

/// Reports whether `matches`, the matches of the deepest command that the
/// caller typed, hold crumbkit's `--dry-run`. The `try_` getter returns an
/// error for an id that the matches do not know or a value of another
/// type, where the plain getter panics, so a tool's own argument under
/// another id is left alone.
pub(crate) fn typed(matches: &ArgMatches) -> bool {
    matches.try_get_one::<bool>(ID).ok().flatten() == Some(&true)
}

impl Output {
    /// Reports whether the run is a dry run: the caller passed `--dry-run`,
    /// which `parse` read, or the tool passed it to [`Output::new`].
    ///
    /// A handler that reads `true` reports what it would change in the
    /// result it would return, and changes nothing. [`finish`](crate::finish)
    /// then marks the result as a dry run. The JSON document holds
    /// `"dry_run": true` right after `schema`. In text, the line `This was a
    /// dry run, so nothing changed.` prints last on standard error, so
    /// standard output holds the result alone. A document without
    /// crumbkit's envelope has no place for the field, so with `--json` that
    /// line also goes to standard error. A failure prints as it does in any
    /// run. [`Output::confirm`] asks nothing in a dry run.
    #[must_use]
    pub const fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}

/// Serializes the flag as an object with `dry_run`, such as
/// `{"dry_run":false}`.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`.
impl Serialize for DryRun {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("dry_run", &self.dry_run)?;
        map.end()
    }
}

/// Describes the object that `Serialize` writes.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature.
impl JsonSchema for DryRun {
    fn schema_name() -> Cow<'static, str> {
        "DryRun".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::DryRun".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {
                "dry_run": {
                    "description": DRY_RUN_HELP,
                    "type": "boolean",
                    "default": false,
                },
            },
            "required": ["dry_run"],
        })
    }
}
