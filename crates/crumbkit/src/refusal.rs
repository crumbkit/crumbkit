//! A refusal: a command line that clap refused, or a flag value that crumbkit
//! refused, printed as a failure with one fixed code and exit code 2.

use std::ffi::OsString;
use std::fmt;
use std::io::Write;
use std::process::ExitCode;

use clap::Command;
use clap::error::{ContextKind, ContextValue, ErrorKind};

use crate::form::FailureForm;
use crate::output::failure;
use crate::page::LimitAbove;
use crate::{Call, Code, Failure, Field, Recovery, Sink, Spelling, codes};

/// A refused command line or flag value, as a [`Failure`] with the code
/// `crumbkit::cli::usage`.
#[derive(Debug)]
pub(crate) struct Refusal {
    /// What the caller got wrong, on one line.
    message: String,
    /// What the caller does next.
    recovery: Recovery,
    /// The other repairs that the caller can try.
    alternatives: Vec<Recovery>,
    /// The values that the caller got wrong, under the names `argument`,
    /// `value` and `subcommand`, in that order.
    fields: Vec<(&'static str, String)>,
    /// The bound of a `--limit` that the refused value passed.
    most: Option<usize>,
}

impl Refusal {
    /// Returns the refusal of a command line that clap refused with
    /// `error`, which says `message` and whose recovery runs the `-h` of the
    /// command at `path`, the names of the subcommands from the root. Its
    /// fields name what `error` names, as [`clap_fields`] reads them.
    ///
    /// When `error` refuses a `--limit` above the bound of a page, and
    /// `words`, the caller's words after the program name, hold that
    /// `--limit`, the recovery runs `words` with `--all` in its place, the
    /// alternative runs them with `--limit` set to the bound, and the field
    /// `most` names the bound (ADR-0050).
    pub(crate) fn command_line(
        message: String,
        path: Vec<String>,
        error: &clap::Error,
        words: &[OsString],
    ) -> Self {
        let fields = clap_fields(error);
        let above =
            std::error::Error::source(error).and_then(|source| source.downcast_ref::<LimitAbove>());
        if let Some(above) = above
            && let Some((all, most)) = replace_limit(words, above.most)
        {
            return Self {
                message,
                recovery: Recovery::Run(Call::typed(all)),
                alternatives: vec![Recovery::Run(Call::typed(most))],
                fields,
                most: Some(above.most),
            };
        }
        Self {
            message,
            recovery: Recovery::Run(Call::help(path)),
            alternatives: Vec::new(),
            fields,
            most: None,
        }
    }

    /// Returns the refusal of the value of the flag `flag` that says
    /// `message` and whose recovery is the hint `hint`, such as `use the
    /// valid fields: a, b`. Its fields name the flag as `argument`, and
    /// `value` when it is `Some`.
    pub(crate) fn flag_value(
        message: String,
        hint: String,
        flag: &'static str,
        value: Option<String>,
    ) -> Self {
        let mut fields = vec![("argument", flag.to_owned())];
        fields.extend(value.map(|value| ("value", value)));
        Self {
            message,
            recovery: Recovery::Hint(hint),
            alternatives: Vec::new(),
            fields,
            most: None,
        }
    }

    /// Prints the refusal to `sink` as `finish` prints a failure, as text or
    /// with `json` as one JSON document, in `form` or in crumbkit's form when
    /// `form` is `None`, and returns exit code 2 (ADR-0002). The recovery
    /// renders against `command`, so it prints as having no next step when
    /// there is no command.
    pub(crate) fn print<O: Write, E: Write, R>(
        &self,
        sink: &mut Sink<O, E, R>,
        json: bool,
        spelling: Spelling,
        form: Option<&dyn FailureForm>,
        command: Option<&mut Command>,
    ) -> ExitCode {
        failure::print(sink, json, spelling, form, self, command);
        ExitCode::from(2)
    }
}

/// Prints the message.
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

/// Returns the fields of a command line that clap refused with `error`:
///
/// - `argument`, the argument that clap names, when it names one, such as
///   `--limit` or `<ID>...`. A flag's text ends before its first space, so
///   the value name in `--limit <N>` is left out;
/// - `value`, the value that clap refused, when it names one;
/// - `subcommand`, the word that names no subcommand, for an unknown
///   subcommand.
///
/// clap gives these through `Error::get`, which needs clap's `error-context`
/// feature (ADR-0003).
//
// RUST-10: cutting `argument` at its first space reads clap's display of an
// argument, which clap does not promise to keep. A flag's name holds no
// space, so the cut keeps the whole name today. Looking the argument up in
// the command by its id replaces the cut once clap's display changes.
fn clap_fields(error: &clap::Error) -> Vec<(&'static str, String)> {
    let mut fields = Vec::new();
    let argument = match error.get(ContextKind::InvalidArg) {
        Some(ContextValue::String(text)) => Some(text),
        Some(ContextValue::Strings(texts)) if texts.len() == 1 => texts.first(),
        _ => None,
    };
    if let Some(argument) = argument {
        let name = match argument.split_once(' ') {
            Some((flag, _)) if flag.starts_with('-') => flag,
            _ => argument,
        };
        fields.push(("argument", name.to_owned()));
    }
    if let Some(ContextValue::String(value)) = error.get(ContextKind::InvalidValue) {
        fields.push(("value", value.clone()));
    }
    // clap also names the parent command under `InvalidSubcommand` when a
    // subcommand is missing, so only an unknown subcommand reads it.
    if error.kind() == ErrorKind::InvalidSubcommand
        && let Some(ContextValue::String(word)) = error.get(ContextKind::InvalidSubcommand)
    {
        fields.push(("subcommand", word.clone()));
    }
    fields
}

impl Failure for Refusal {
    fn code(&self) -> &'static Code {
        &codes::CLI_USAGE
    }

    fn recovery(&self) -> Recovery {
        self.recovery.clone()
    }

    fn alternatives(&self) -> Vec<Recovery> {
        self.alternatives.clone()
    }

    fn fields(&self) -> Vec<Field<'_>> {
        let mut fields: Vec<Field<'_>> = self
            .fields
            .iter()
            .map(|(name, value)| Field::new(name, value))
            .collect();
        fields.extend(self.most.map(|most| Field::new("most", most)));
        fields
    }
}

/// Returns `words` twice, with their first `--limit` and its value replaced:
/// once by `--all`, and once by `--limit` with `most`. It returns `None`
/// when no word before any `--` is `--limit` with a value after it or
/// `--limit=` with its value.
//
// RUST-10: this reads only the spellings `--limit N` and `--limit=N`. An
// alias that a tool adds to `--limit` leaves the recovery at the `-h` of
// the command. Reading the flag's position from clap's matches replaces this
// if a tool needs the alias.
fn replace_limit(words: &[OsString], most: usize) -> Option<(Vec<String>, Vec<String>)> {
    let words: Vec<String> = words
        .iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect();
    let end = words
        .iter()
        .position(|word| word == "--")
        .unwrap_or(words.len());
    let (start, width) = words
        .get(..end)?
        .iter()
        .enumerate()
        .find_map(|(index, word)| {
            if word == "--limit" && index + 1 < end {
                Some((index, 2))
            } else if word.starts_with("--limit=") {
                Some((index, 1))
            } else {
                None
            }
        })?;
    let before = words.get(..start)?;
    let after = words.get(start + width..)?;
    let all = [before, &["--all".to_owned()], after].concat();
    let limited = [before, &["--limit".to_owned(), most.to_string()], after].concat();
    Some((all, limited))
}
