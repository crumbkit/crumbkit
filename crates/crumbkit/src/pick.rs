//! Pickers, which let a person pick the value of a missing argument from a
//! list when a command line leaves out a required argument (ADR-0043).
//!
//! A tool sets its picker with [`Guide::picker`](crate::help::Guide::picker)
//! and names where each argument's values come from with
//! [`Leaf::choices`](crate::help::Leaf::choices). An argument with possible
//! values needs no source, because clap already knows its values.
//! [`parse`](crate::parse) asks the picker only when a person can answer,
//! which means that standard input and standard error are terminals and no
//! coding agent runs the tool, and only when the caller did not pass
//! `--no-input`.

use std::fmt;
use std::io::{self, BufRead, Write};

use serde::Serialize;
use serde_json::Value;

pub use self::no_input::NoInput;
use crate::output::table::{cell, escape_controls};
use crate::{Error, Row};

pub(crate) mod no_input;
#[cfg(test)]
mod tests;

/// A way to ask a person to pick one or more values from a list.
///
/// crumbkit-picker's `Menu` is a picker that needs only the standard
/// library. A tool can wrap any other prompt library in its own type that
/// implements this trait.
pub trait Picker {
    /// Asks the person to pick between `question.least()` and
    /// `question.most()` of `question.choices()`, reading the person's
    /// answer from `input` and writing the list and any prompt to `output`,
    /// which is the process's standard error in a real run.
    ///
    /// It returns the positions of the picked choices in
    /// `question.choices()`, or [`Answer::Cancelled`] when the person
    /// picked nothing. `parse` refuses the command line as it would without
    /// a picker when the answer is cancelled, names a position outside the
    /// list, or picks a number of choices outside the question's count.
    ///
    /// # Errors
    ///
    /// Returns the error of a failed read or write. `parse` then refuses
    /// the command line as it would without a picker.
    fn pick(
        &self,
        question: &Question<'_>,
        input: &mut dyn BufRead,
        output: &mut dyn Write,
    ) -> io::Result<Answer>;
}

/// Prints `Picker`, so a [`Guide`](crate::help::Guide) that holds a picker
/// can derive `Debug`.
impl fmt::Debug for dyn Picker + '_ {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Picker")
    }
}

/// What a person answered to a [`Question`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Answer {
    /// The positions of the picked choices in the question's list, in the
    /// order the person picked them.
    Picked(Vec<usize>),
    /// The person picked nothing.
    Cancelled,
}

/// The question that `parse` asks a [`Picker`] about one missing argument.
#[derive(Clone, Copy, Debug)]
pub struct Question<'a> {
    /// The command line of the command, such as `shelf books show`.
    command: &'a str,
    /// The argument's value name, such as `ID`.
    name: &'a str,
    /// The argument's help text, or `None` when it has none.
    help: Option<&'a str>,
    /// The values that the person can pick from.
    choices: &'a [Choice],
    /// The fewest choices that the person must pick.
    least: usize,
    /// The most choices that the person may pick.
    most: usize,
}

impl<'a> Question<'a> {
    /// Returns the question that asks for exactly one of `choices` as the
    /// value `name` of the command `command`, without help text.
    #[must_use]
    pub const fn new(command: &'a str, name: &'a str, choices: &'a [Choice]) -> Self {
        Self {
            command,
            name,
            help: None,
            choices,
            least: 1,
            most: 1,
        }
    }

    /// Sets the argument's help text.
    #[must_use]
    pub const fn help(mut self, help: &'a str) -> Self {
        self.help = Some(help);
        self
    }

    /// Sets the fewest and the most choices that the person picks.
    /// `usize::MAX` as `most` means any number.
    #[must_use]
    pub const fn count(mut self, least: usize, most: usize) -> Self {
        self.least = least;
        self.most = most;
        self
    }

    /// Returns the command line of the command, such as `shelf books show`.
    #[must_use]
    pub const fn command(&self) -> &'a str {
        self.command
    }

    /// Returns the argument's value name, such as `ID`.
    #[must_use]
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the help text that [`Question::help`] set. The name has
    /// clap's `get_` prefix, because `help` is the builder method.
    #[must_use]
    pub const fn get_help(&self) -> Option<&'a str> {
        self.help
    }

    /// Returns the values that the person can pick from.
    #[must_use]
    pub const fn choices(&self) -> &'a [Choice] {
        self.choices
    }

    /// Returns the fewest choices that the person must pick.
    #[must_use]
    pub const fn least(&self) -> usize {
        self.least
    }

    /// Returns the most choices that the person may pick, which is
    /// `usize::MAX` when there is no limit.
    #[must_use]
    pub const fn most(&self) -> usize {
        self.most
    }
}

/// One value that a person can pick, with the label that a picker shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The word that `parse` adds to the command line.
    value: String,
    /// The text that a picker shows for the value.
    label: String,
}

impl Choice {
    /// Returns the choice of `value`, the word that `parse` adds to the
    /// command line, shown as `label`.
    #[must_use]
    pub const fn new(value: String, label: String) -> Self {
        Self { value, label }
    }

    /// Returns the word that `parse` adds to the command line.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the text that a picker shows for the value.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// The source of the values of one argument of a leaf, which
/// [`Leaf::choices`](crate::help::Leaf::choices) sets.
///
/// `parse` calls the source only when it asks a picker about the argument.
/// A failed source ends the run with exit code 1 after `parse` prints its
/// failure. An argument with possible values uses them and never calls its
/// source.
#[derive(Clone, Copy, Debug)]
pub struct Choices {
    /// The clap id of the argument.
    arg: &'static str,
    /// The function that returns the values.
    source: fn() -> Result<Vec<Choice>, Error>,
}

impl Choices {
    /// Returns the source of the values of the argument whose clap id is
    /// `arg`, which `source` returns.
    #[must_use]
    pub const fn new(arg: &'static str, source: fn() -> Result<Vec<Choice>, Error>) -> Self {
        Self { arg, source }
    }

    /// Returns the clap id of the argument.
    #[must_use]
    pub const fn arg(&self) -> &'static str {
        self.arg
    }

    /// Calls the source and returns the values.
    ///
    /// # Errors
    ///
    /// Returns the source's failure.
    pub fn load(&self) -> Result<Vec<Choice>, Error> {
        (self.source)()
    }
}

/// Returns one choice for each of `rows`, for a source that offers the rows
/// that a list command prints.
///
/// The value of each choice is the row's field `id`, where a string stays
/// as it is and any other value is its JSON form. The label is the row's
/// other [`Row::COLUMNS`], each as a table cell shows it, joined with two
/// spaces, or the value when the row has no other column. A row that does
/// not serialize, or whose field `id` is missing or `null`, has no choice.
pub fn from_rows<T: Row + Serialize>(rows: &[T], id: &str) -> Vec<Choice> {
    rows.iter()
        .filter_map(|row| {
            let row = serde_json::to_value(row).ok()?;
            let value = match row.get(id)? {
                Value::Null => return None,
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            let label = T::COLUMNS
                .iter()
                .filter(|&&name| name != id)
                .map(|name| cell(row.get(name)))
                .collect::<Vec<_>>()
                .join("  ");
            let label = if label.is_empty() {
                escape_controls(&value).into_owned()
            } else {
                label
            };
            Some(Choice::new(value, label))
        })
        .collect()
}
