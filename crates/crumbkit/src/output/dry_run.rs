//! `--dry-run`, which asks a command to report what it would change and
//! change nothing.

use std::borrow::Cow;

use clap::Args;
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

/// The flag `--dry-run`, which asks a command to report what it would change
/// and change nothing.
///
/// A command that changes something adds it to its own arguments struct
/// with `#[command(flatten)]` and passes it to [`Output::dry_run`], which
/// records it and returns whether the run is a dry run:
///
/// ```
/// use clap::Parser;
/// use crumbkit::{DryRun, Output};
///
/// /// Remove books from the catalogue.
/// #[derive(Parser)]
/// struct BooksRemove {
///     #[command(flatten)]
///     dry_run: DryRun,
///     #[command(flatten)]
///     output: Output,
/// }
///
/// let mut args = BooksRemove::parse_from(["books-remove", "--dry-run"]);
/// assert!(args.output.dry_run(args.dry_run));
/// ```
///
/// The flag's clap id is `crumbkit_dry_run`, so crumbkit-test's
/// `check_effects` tells it apart from a tool's own `--dry-run`. It has no
/// getter, so a tool reads it only through `Output::dry_run`, and `finish`
/// always knows when a run is a dry run.
#[derive(Args, Clone, Copy, Debug, Default, PartialEq, Eq)]
// clap builds the arguments of every command on every run, so the type
// skips the argument group that clap would build for its one flag.
#[group(skip)]
pub struct DryRun {
    #[arg(id = "crumbkit_dry_run", long = "dry-run", help = DRY_RUN_HELP)]
    dry_run: bool,
}

impl Output {
    /// Records whether the caller passed `--dry-run`, and returns it.
    ///
    /// A handler that receives `true` reports what it would change in the
    /// result it would return, and changes nothing. [`finish`](crate::finish)
    /// then marks the result as a dry run. The JSON document holds
    /// `"dry_run": true` right after `schema`. In text, the line `This was a
    /// dry run, so nothing changed.` prints last on standard error, so
    /// standard output holds the result alone. A document without
    /// crumbkit's envelope has no place for the field, so with `--json` that
    /// line also goes to standard error. A failure prints as it does in any
    /// run. [`Output::confirm`] asks nothing in a dry run.
    pub fn dry_run(&mut self, flag: DryRun) -> bool {
        self.dry_run = flag.dry_run;
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
