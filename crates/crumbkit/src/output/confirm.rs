//! `--yes`, and `Output::confirm`, which asks before a command destroys
//! data.

use std::borrow::Cow;
use std::ffi::OsString;
use std::fmt;
use std::io::{BufRead, Write};

use clap::Args;
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

use super::Output;
use crate::{Call, Code, Error, Failure, Recovery, Sink, codes};

#[cfg(test)]
mod tests;

/// The help text of `--yes`, which the schema also uses as its description.
const YES_HELP: &str = "Proceed without asking for confirmation";

/// The flag `--yes`, which lets a command that asks for confirmation proceed
/// without asking.
///
/// A command that destroys data adds it to its own arguments struct with
/// `#[command(flatten)]` and passes it to [`Output::confirm`].
/// crumbkit-test's `check_effects` requires `--yes` on every command whose
/// [`Effects`](crate::help::Effects) are destructive.
#[derive(Args, Clone, Copy, Debug, Default, PartialEq, Eq)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
// clap builds the arguments of every command on every run, so the type
// skips the argument group that clap would build for its one flag.
#[group(skip)]
pub struct Yes {
    #[arg(long, help = YES_HELP)]
    yes: bool,
}

impl Output {
    /// Asks the person at the terminal `question` before the command
    /// changes anything, as [`Output::confirm_to`] does with the process's
    /// standard error and standard input.
    ///
    /// # Errors
    ///
    /// Returns the failures that [`Output::confirm_to`] describes.
    pub fn confirm(&mut self, yes: Yes, question: &str) -> Result<(), Error> {
        // With terminal detection off, `confirm_to` never asks, so the run
        // neither locks standard input nor detects a terminal or an agent.
        if self.no_terminal {
            return self.confirm_to(&mut Sink::standard(), yes, question);
        }
        self.confirm_to(&mut Sink::standard_with_input(), yes, question)
    }

    /// Asks `question` through `sink` before the command changes anything,
    /// and returns `Ok` when the command may proceed.
    ///
    /// It returns `Ok` without asking when the caller passed `--yes`, or
    /// when [`Output::dry_run`] recorded a dry run, which changes nothing.
    /// Otherwise, when the sink is interactive, it writes the question and
    /// ` [y/N] ` to the sink's error writer and reads one line. The answer
    /// `y` or `yes`, in any case and with any surrounding spaces, proceeds.
    ///
    /// A sink from [`Sink::new`] is never interactive, and the sink of
    /// [`Output::confirm`] is interactive when standard input and standard
    /// error are both terminals and no coding agent runs the tool. A run
    /// whose caller passed [`NoInput`](crate::NoInput)'s `--no-input` never
    /// asks.
    ///
    /// # Errors
    ///
    /// Returns a failure with exit code 1 from `finish`:
    ///
    /// - `<tool>::confirmation::declined`, when the answer is anything else or
    ///   standard input ends first. It has no next step, because a person
    ///   chose not to proceed.
    /// - `<tool>::confirmation::needs_yes`, when the sink is not interactive,
    ///   the caller passed `--no-input`, or the tool's guide turned terminal
    ///   detection off, so no person can answer. Its recovery runs the
    ///   words that the caller typed with `--yes` added before any `--`,
    ///   such as
    ///   `shelf books remove 2 --yes`. The recovery has no next step when a
    ///   typed word is not UTF-8.
    pub fn confirm_to<O: Write, E: Write, I: BufRead>(
        &mut self,
        sink: &mut Sink<O, E, I>,
        yes: Yes,
        question: &str,
    ) -> Result<(), Error> {
        if yes.yes || self.dry_run {
            return Ok(());
        }
        if self.no_input || self.no_terminal || !sink.interactive {
            return Err(ConfirmError::NeedsYes(self.words.clone()).into());
        }
        // A prompt that cannot be written or an answer that cannot be read
        // leaves no answer, which declines.
        let _ = write!(sink.err, "{question} [y/N] ");
        let _ = sink.err.flush();
        let mut answer = String::new();
        let _ = sink.input.read_line(&mut answer);
        let answer = answer.trim();
        if answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
            Ok(())
        } else {
            Err(ConfirmError::Declined.into())
        }
    }
}

/// A failure of [`Output::confirm`].
#[derive(Debug)]
enum ConfirmError {
    /// The person at the terminal did not answer yes.
    Declined,
    /// No person can answer. It holds the words that the caller typed after
    /// the program name.
    NeedsYes(Vec<OsString>),
}

impl fmt::Display for ConfirmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Declined => f.write_str("the answer was not yes, so the command changed nothing"),
            Self::NeedsYes(_) => f.write_str(
                "the command asks for confirmation, and no person can answer at a terminal, so it changed nothing",
            ),
        }
    }
}

impl std::error::Error for ConfirmError {}

impl Failure for ConfirmError {
    fn code(&self) -> &'static Code {
        match self {
            Self::Declined => &codes::CONFIRM_DECLINED,
            Self::NeedsYes(_) => &codes::CONFIRM_NEEDS_YES,
        }
    }

    fn recovery(&self) -> Recovery {
        match self {
            Self::Declined => Recovery::None {
                reason: "the person at the terminal chose not to proceed".to_owned(),
            },
            Self::NeedsYes(words) => match with_yes(words) {
                Some(words) => Recovery::Run(Call::typed(words)),
                None => Recovery::None {
                    reason: "a word of the command line is not UTF-8, so the command line cannot be repeated with --yes"
                        .to_owned(),
                },
            },
        }
    }
}

/// Returns `words` with `--yes` added before the first `--`, or at the end
/// when there is none, or `None` when a word is not UTF-8.
fn with_yes(words: &[OsString]) -> Option<Vec<String>> {
    let mut words = words
        .iter()
        .map(|word| word.to_str().map(str::to_owned))
        .collect::<Option<Vec<String>>>()?;
    let at = words
        .iter()
        .position(|word| word == "--")
        .unwrap_or(words.len());
    words.insert(at, "--yes".to_owned());
    Some(words)
}

/// Serializes the flag as an object with `yes`, such as `{"yes":false}`.
///
/// This impl is written by hand so crumbkit needs no `serde_derive`.
impl Serialize for Yes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry("yes", &self.yes)?;
        map.end()
    }
}

/// Describes the object that `Serialize` writes.
///
/// This impl is written by hand so crumbkit needs no schemars `derive`
/// feature.
impl JsonSchema for Yes {
    fn schema_name() -> Cow<'static, str> {
        "Yes".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "crumbkit::Yes".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "object",
            "properties": {
                "yes": {
                    "description": YES_HELP,
                    "type": "boolean",
                    "default": false,
                },
            },
            "required": ["yes"],
        })
    }
}
