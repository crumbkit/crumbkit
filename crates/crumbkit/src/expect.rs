//! `--expect`, which makes a write fail with a conflict unless its target is
//! at the version that the caller read (ADR-0047).

use std::borrow::Cow;
use std::fmt;

use clap::builder::NonEmptyStringValueParser;
use clap::{Args, Command};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use crate::{Call, Code, Error, Failure, Field, Recovery, codes};

#[cfg(test)]
mod tests;

/// The clap id of crumbkit's `--expect`, which tells it apart from a tool's
/// own flag with the same name.
pub(crate) const EXPECT_ID: &str = "crumbkit_expect";

/// The help text of `--expect`, which the schema also uses as its
/// description.
const EXPECT_HELP: &str =
    "Fail with a conflict unless the target is at VERSION, or does not exist when VERSION is none";

/// The word that `--expect` reads as a target that must not exist.
const NONE: &str = "none";

/// The option `--expect <VERSION>`, which makes a write fail with a conflict
/// unless the target is at the version that the caller read, so two
/// concurrent writers cannot lose each other's changes unnoticed.
///
/// A command that writes adds it to its own arguments struct with
/// `#[command(flatten)]`, and before it changes anything it passes the
/// target's current version to [`Expect::check`] with the command that
/// reads the target:
///
/// ```
/// use clap::Parser;
/// use crumbkit::{Call, Expect};
///
/// /// Rename a planet.
/// #[derive(Parser)]
/// struct PlanetsRename {
///     name: String,
///     #[command(flatten)]
///     expect: Expect,
/// }
///
/// let args = PlanetsRename::parse_from(["planets-rename", "Pluto", "--expect", "7"]);
/// let read = Call::new(&["planets", "show"]);
/// assert!(args.expect.check(Some("7"), read.clone()).is_ok());
/// assert!(args.expect.check(Some("8"), read).is_err());
/// ```
///
/// The tool decides what a version is, such as a revision number or a hash
/// of the target, and prints it in the output of the command that reads the
/// target. The word `none` is reserved: it means that the target must not
/// exist, so a tool whose versions could be spelled `none` spells them
/// differently. The caller may leave `--expect` out, and the write then has
/// no precondition. The option refuses an empty version.
///
/// The option's clap id is `crumbkit_expect`. Help and `commands list` read
/// it from the command tree to say that the command takes `--expect`, and
/// crumbkit-test's `check_effects` requires it on a command whose
/// [`Effects`](crate::help::Effects) are
/// [`replacing`](crate::help::Effects::replacing).
#[derive(Args, Clone, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
// clap builds the arguments of every command on every run, so the type
// skips the argument group that clap would build for its one option.
#[group(skip)]
pub struct Expect {
    #[arg(
        id = "crumbkit_expect",
        long = "expect",
        value_name = "VERSION",
        help = EXPECT_HELP,
        value_parser = NonEmptyStringValueParser::new()
    )]
    expect: Option<String>,
}

impl Expect {
    /// Returns `Ok` when the target's `current` version, or `None` when the
    /// target does not exist, is what the caller expects, and the conflict
    /// of [`Expect::conflict`] otherwise. Without `--expect` every target
    /// passes. The comparison is exact, so a tool whose versions have more
    /// than one spelling passes the spelling that its read command prints.
    ///
    /// # Errors
    ///
    /// Returns the failure `<tool>::version::conflict`, whose recovery
    /// runs `read`, when the versions differ, when `--expect none` meets a
    /// target that exists, or when a version is expected and no target
    /// exists.
    pub fn check(&self, current: Option<&str>, read: Call) -> Result<(), Error> {
        let passes = match (self.expect.as_deref(), current) {
            (None, _) | (Some(NONE), None) => true,
            (Some(NONE), Some(_)) | (Some(_), None) => false,
            (Some(expected), Some(current)) => expected == current,
        };
        if passes {
            Ok(())
        } else {
            Err(self.conflict(current, read))
        }
    }

    /// Returns the failure `<tool>::version::conflict` for a target whose
    /// `current` version, or `None` when it does not exist, is not what the
    /// caller expects. A tool that compares versions in its own way, such
    /// as numerically, calls it in place of [`Expect::check`].
    ///
    /// The failure exits with code 1, its recovery runs `read`, the command
    /// that prints the target's current version, and its fields name the
    /// `expected` version, when the caller passed one, and the `current`
    /// version, which is `none` for a target that does not exist.
    #[must_use]
    pub fn conflict(&self, current: Option<&str>, read: Call) -> Error {
        Conflict {
            expected: self.expect.clone(),
            current: current.map(str::to_owned),
            read,
        }
        .into()
    }
}

/// Reports whether `command` takes crumbkit's `--expect`.
pub(crate) fn takes_expect(command: &Command) -> bool {
    command
        .get_arguments()
        .any(|arg| arg.get_id() == EXPECT_ID && arg.get_long() == Some("expect"))
}

/// A target that is not at the version that the caller expects.
#[derive(Debug)]
struct Conflict {
    /// The word that the caller passed to `--expect`, or `None` when the
    /// caller passed none.
    expected: Option<String>,
    /// The target's current version, or `None` when it does not exist.
    current: Option<String>,
    /// The command that prints the target's current version.
    read: Call,
}

impl fmt::Display for Conflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self.current.as_deref();
        match (self.expected.as_deref(), current) {
            (Some(NONE), Some(current)) => write!(
                f,
                "the target exists at version {current}, and --expect none requires that it does not exist, so the command changed nothing"
            ),
            (Some(expected), Some(current)) => write!(
                f,
                "the target is at version {current}, and --expect named version {expected}, so the command changed nothing"
            ),
            (Some(expected), None) => write!(
                f,
                "the target does not exist, and --expect named version {expected}, so the command changed nothing"
            ),
            (None, _) => f.write_str(
                "the target is not at the version that the caller read, so the command changed nothing",
            ),
        }
    }
}

impl std::error::Error for Conflict {}

impl Failure for Conflict {
    fn code(&self) -> &'static Code {
        &codes::VERSION_CONFLICT
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(self.read.clone())
    }

    fn fields(&self) -> Vec<Field<'_>> {
        let mut fields = Vec::new();
        if let Some(expected) = &self.expected {
            fields.push(Field::new("expected", expected.as_str()));
        }
        fields.push(Field::new(
            "current",
            self.current.as_deref().unwrap_or(NONE),
        ));
        fields
    }
}

/// Serializes the option as an object with `expect`, such as
/// `{"expect":"7"}`, or `{"expect":null}` when the caller left it out.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`.
impl Serialize for Expect {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("expect", &self.expect)?;
        map.end()
    }
}

/// Describes the object that `Serialize` writes.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature.
impl JsonSchema for Expect {
    fn schema_name() -> Cow<'static, str> {
        "Expect".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::Expect".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {
                "expect": {
                    "description": EXPECT_HELP,
                    "type": ["string", "null"],
                    "minLength": 1,
                },
            },
            "required": ["expect"],
        })
    }
}
