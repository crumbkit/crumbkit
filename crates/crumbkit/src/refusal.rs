//! A refusal: a command line that clap refused, or a flag value that crumbkit
//! refused, printed as a failure with crumbkit's usage code, or the tool's
//! rename of it, and exit code 2 unless the guide lists another.

use std::ffi::OsString;
use std::fmt;

use clap::builder::StyledStr;
use clap::error::{ContextKind, ContextValue, ErrorKind};
use clap::{Arg, ArgAction, Command};

use crate::output::failure::{self, PreparedFailure, Printing};
use crate::page::LimitAbove;
use crate::sink::role::{Deliver, Message};
use crate::{Call, Code, Failure, Field, Recovery, Spelling, codes};

mod styled;

/// A refused command line or flag value, as a [`Failure`] with the code
/// `<tool>::cli::usage`.
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
    /// clap's usage lines, each without clap's `Usage:` label, which the
    /// text form prints under the message. A refusal that crumbkit words
    /// itself has none.
    usage: Vec<String>,
    /// The help page of the group that the refused command line stopped
    /// at, which the text form prints in place of its lines, or `None`.
    page: Option<StyledStr>,
    /// clap's rendered error, whose styles the text form takes at a sink
    /// whose error writer wants styles (ADR-0131), or `None` for a refusal
    /// that crumbkit words itself.
    clap: Option<StyledStr>,
}

impl Refusal {
    /// Returns the refusal of a command line that clap refused with
    /// `error`, which says `message`. Its recovery is what
    /// [`help_recovery`] returns for the command at `path` of `command`, the
    /// names of the subcommands from the root, unless `declared` below
    /// holds a call. Its fields name what `error` names, as
    /// [`clap_fields`] reads them.
    ///
    /// When `error` refuses a `--limit` above the bound of a page, and
    /// `words`, the caller's words after the program name, hold that
    /// `--limit`, the recovery runs `words` with `--all` in its place, the
    /// alternative runs them with `--limit` set to the bound, and the field
    /// `most` names the bound (ADR-0050).
    ///
    /// `declared` holds the calls that the function of the leaf at `path`
    /// returned and that render and parse, in the function's order, and
    /// `hint` the sentence that the leaf's hint function returned. For a
    /// `--limit` above its bound the hint and then the calls follow
    /// crumbkit's own alternatives (ADR-0056, ADR-0123). Otherwise a hint
    /// becomes the recovery, and the `-h` call becomes the first
    /// alternative, followed by the calls (ADR-0123). Without a hint, the
    /// first call becomes the recovery, and the `-h` call becomes the first
    /// alternative, followed by the rest of them (ADR-0078). With neither,
    /// the help recovery stays the recovery. A path without a help page has
    /// no `-h` call to offer as an alternative (ADR-0124).
    pub(crate) fn command_line(
        message: String,
        command: &Command,
        path: &[String],
        error: &clap::Error,
        words: &[OsString],
        declared: Vec<Call>,
        hint: Option<String>,
    ) -> Self {
        let fields = clap_fields(error);
        let mut declared = declared.into_iter().map(Recovery::Run);
        let hint = hint.map(Recovery::Hint);
        let above =
            std::error::Error::source(error).and_then(|source| source.downcast_ref::<LimitAbove>());
        if let Some(above) = above
            && let Some((all, most)) = replace_limit(words, above.most)
        {
            return Self {
                message,
                recovery: Recovery::Run(Call::typed(all)),
                alternatives: std::iter::once(Recovery::Run(Call::typed(most)))
                    .chain(hint)
                    .chain(declared)
                    .collect(),
                fields,
                most: Some(above.most),
                usage: Vec::new(),
                page: None,
                clap: None,
            };
        }
        let help = help_recovery(command, path);
        let (recovery, alternatives) = match hint.or_else(|| declared.next()) {
            Some(first) => {
                let help = Some(help).filter(|help| matches!(help, Recovery::Run(_)));
                (first, help.into_iter().chain(declared).collect())
            }
            None => (help, Vec::new()),
        };
        Self {
            message,
            recovery,
            alternatives,
            fields,
            most: None,
            usage: Vec::new(),
            page: None,
            clap: None,
        }
    }

    /// Returns the refusal with clap's usage lines `usage`, which its text
    /// form prints under the message, and the help page `page` of the group
    /// that the command line stopped at, which its text form prints in
    /// place of its lines. The failure document prints neither.
    pub(crate) fn shown_with(mut self, usage: Vec<String>, page: Option<StyledStr>) -> Self {
        self.usage = usage;
        self.page = page;
        self
    }

    /// Returns the refusal with `rendered`, clap's rendered error for the
    /// refused command line, whose styles its text form takes at a sink
    /// whose error writer wants styles (ADR-0131).
    pub(crate) fn styled_as(mut self, rendered: StyledStr) -> Self {
        self.clap = Some(rendered);
        self
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
            usage: Vec::new(),
            page: None,
            clap: None,
        }
    }

    /// Returns what the caller got wrong, on one line.
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    /// Returns the value of the field called `name`, one of `argument`,
    /// `value` and `subcommand`, or `None` when the refusal has no such
    /// field.
    pub(crate) fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(field, _)| *field == name)
            .map(|(_, value)| value.as_str())
    }

    /// Prints the refusal to `sink` as `finish` prints a failure, as text or
    /// with `json` as one JSON document, as `printing` decides, and returns
    /// the exit code that `printing` gives the usage code: 2 unless the
    /// guide lists another (ADR-0002). The recovery renders against
    /// `command`.
    ///
    /// The text form prints clap's usage lines under the message. A refusal
    /// with a group's help page prints that page as its text form in place
    /// of the lines, and reports its code to the observer as every refusal
    /// does (ADR-0118).
    ///
    /// At a sink whose error writer wants styles, the text form carries
    /// clap's styles, as [`styled::text`] adds them with the command's
    /// error style, and the group's page keeps clap's styles (ADR-0131).
    pub(crate) fn print(
        &self,
        sink: &mut dyn Deliver,
        json: bool,
        spelling: Spelling,
        printing: Printing,
        command: &mut Command,
    ) -> u8 {
        let error = *command.get_styles().get_error();
        let restyle = |plain: &str| styled::text(plain, &error, self.clap.as_ref());
        let document =
            PreparedFailure::new(self, spelling, printing, command, sink).with_usage(&self.usage);
        failure::print_prepared(
            sink,
            json,
            printing.layout,
            &document,
            Message::Refusal,
            self.page.as_ref(),
            Some(&restyle),
        );
        printing.exit(&codes::CLI_USAGE)
    }
}

/// Prints the message.
impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Refusal {}

/// The reason of the recovery of a refusal whose typed path holds no
/// command with a help page.
const NO_HELP: &str = "no command that the command line names has a -h help page";

/// Returns the help recovery of a refusal of a command line whose deepest
/// typed command is at `path` of `command`, the names of the subcommands
/// from the root.
///
/// The recovery runs the `-h` of the deepest command on the path that has a
/// help flag, from the deepest typed command up to the root. A hidden
/// command counts. A command has a help flag when neither it nor a command
/// above it turned clap's flag off with `disable_help_flag`, which clap
/// applies to every command below the one that sets it. A command also has
/// one when it declares its own `-h` argument that prints help, or a command
/// above it declares one as a global argument. When no command on the path
/// has a help flag, the recovery is `Recovery::None` with [`NO_HELP`], so a
/// refusal never names a call that clap refuses (ADR-0124). A name that
/// `command` does not hold ends the path there.
fn help_recovery(command: &Command, path: &[String]) -> Recovery {
    let mut disabled = false;
    let mut global = false;
    let mut deepest = None;
    let mut node = Some(command);
    for depth in 0..=path.len() {
        let Some(current) = node else {
            break;
        };
        disabled |= current.is_disable_help_flag_set();
        let own = current.get_arguments().find(|arg| prints_help(arg));
        global |= own.is_some_and(Arg::is_global_set);
        if !disabled || global || own.is_some() {
            deepest = Some(depth);
        }
        node = path
            .get(depth)
            .and_then(|name| current.find_subcommand(name));
    }
    match deepest.and_then(|depth| path.get(..depth)) {
        Some(path) => Recovery::Run(Call::help(path.to_vec())),
        None => Recovery::None {
            reason: NO_HELP.to_owned(),
        },
    }
}

/// Reports whether `arg` is a `-h` flag that prints help.
fn prints_help(arg: &Arg) -> bool {
    arg.get_short() == Some('h')
        && matches!(
            arg.get_action(),
            ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong
        )
}

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
