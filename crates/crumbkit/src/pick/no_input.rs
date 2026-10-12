//! `--no-input`, which tells a tool that no person can answer a question.

use std::borrow::Cow;

use clap::Args;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

#[cfg(test)]
mod tests;

/// The clap id of `--no-input`, which `parse` reads from the matches and
/// crumbkit-test's `check_effects` looks for.
pub(crate) const NO_INPUT_ID: &str = "crumbkit_no_input";

/// The help text of `--no-input`, which the schema also uses as its
/// description.
const NO_INPUT_HELP: &str = "Fail instead of asking a question";

/// The global flag `--no-input`, which tells the tool that no person can
/// answer a question, as a script or an agent that runs it can say.
///
/// A tool flattens it into its root arguments struct once, and every
/// command accepts it:
///
/// ```
/// use clap::Parser;
/// use crumbkit::NoInput;
///
/// /// Browse a small catalogue.
/// #[derive(Parser)]
/// struct Cli {
///     #[command(flatten)]
///     no_input: NoInput,
/// }
///
/// let cli = Cli::parse_from(["shelf", "--no-input"]);
/// assert_ne!(cli.no_input, NoInput::default());
/// ```
///
/// With the flag, [`parse`](crate::parse) never asks a picker for a missing
/// argument and refuses the command line instead, and
/// [`Output::confirm`](crate::Output::confirm) fails with
/// `crumbkit::confirmation::needs_yes` instead of asking. The flag's clap id is
/// `crumbkit_no_input`. `parse` reads it from the matches, so the type has
/// no getter. crumbkit-test's `check_effects` requires the flag at the root
/// of a tool that has a picker or a `--yes` (ADR-0043).
#[derive(Args, Clone, Copy, Debug, Default, PartialEq, Eq)]
// clap builds the arguments of every command on every run, so the type
// skips the argument group that clap would build for its one flag.
#[group(skip)]
pub struct NoInput {
    #[arg(id = "crumbkit_no_input", long = "no-input", global = true, help = NO_INPUT_HELP)]
    no_input: bool,
}

/// Serializes the flag as an object with `no_input`, such as
/// `{"no_input":false}`.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`.
impl Serialize for NoInput {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("no_input", &self.no_input)?;
        map.end()
    }
}

/// Describes the object that `Serialize` writes.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature.
impl JsonSchema for NoInput {
    fn schema_name() -> Cow<'static, str> {
        "NoInput".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::NoInput".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {
                "no_input": {
                    "description": NO_INPUT_HELP,
                    "type": "boolean",
                    "default": false,
                },
            },
            "required": ["no_input"],
        })
    }
}
