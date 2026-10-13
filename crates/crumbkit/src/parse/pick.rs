//! Picking, which asks the guide's picker for the values of a required
//! argument that the command line leaves out, and adds them to the words
//! (ADR-0043).

use std::ffi::OsString;
use std::io::{BufRead, Write};

use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Command};

use super::walk::given;
use crate::help::{Guide, Leaf, quote};
use crate::output::failure::{self, Printing};
use crate::pick::no_input::NO_INPUT_ID;
use crate::pick::{Answer, Choice, Choices, Picker, Question};
use crate::{Sink, Spelling};

#[cfg(test)]
mod tests;

/// What one [`Picking::pick`] did.
pub(super) enum Step {
    /// The picker picked values, which are now in the words.
    Picked,
    /// Nothing was picked, and `parse` refuses the command line as it would
    /// without a picker.
    Refuse,
    /// A source failed, its failure is printed, and the run ends with the
    /// exit code.
    Stop(u8),
}

/// The picking of one run, which asks about each missing argument at most
/// once.
pub(super) struct Picking {
    /// The guide's picker.
    picker: &'static dyn Picker,
    /// Whether the run reads the process's standard input, as `parse` and
    /// `parse_from` do, instead of the input of the sink that `parse_to`
    /// receives.
    standard: bool,
    /// The clap ids of the arguments that the picker was asked about.
    asked: Vec<String>,
}

impl Picking {
    /// Returns the picking of a run with `picker`, which reads the
    /// process's standard input when `standard`.
    pub(super) const fn new(picker: &'static dyn Picker, standard: bool) -> Self {
        Self {
            picker,
            standard,
            asked: Vec::new(),
        }
    }

    /// Asks the picker about the first missing required argument of the
    /// leaf that `words` name in `command`, and adds the picked values to
    /// `words`.
    ///
    /// It refuses without asking when the words name no leaf, when the
    /// caller passed `--no-input`, when no person can answer, when any
    /// missing required argument has neither possible values nor a source
    /// in the leaf's [`Leaf::choices`], when the picker was already asked
    /// about the argument, or when the argument has no choices. It refuses
    /// after asking when the answer is cancelled, names a position outside
    /// the list, or picks a number of values outside the argument's count.
    /// A failed source prints its failure with `json` and `spelling`, and
    /// stops the run with the exit code that the guide gives its code, which
    /// is 1 unless an [`Exit`](crate::help::Exit) lists it.
    //
    // RUST-10: an argument counts as required only when clap marks it
    // `required`. An argument that `required_unless_present`,
    // `required_if_eq` or a required group requires is missing only under
    // conditions that the walk does not check, so its command line is
    // refused as it is without a picker. Reading clap's own list of the
    // missing arguments from the error replaces this if a tool needs
    // picking for those.
    pub(super) fn pick<O: Write, E: Write, R: BufRead>(
        &mut self,
        sink: &mut Sink<O, E, R>,
        command: &mut Command,
        guide: &Guide,
        words: &mut Vec<OsString>,
        json: bool,
        spelling: Spelling,
    ) -> Step {
        let given = given(command, words);
        if given.ids.iter().any(|id| id == NO_INPUT_ID) {
            return Step::Refuse;
        }
        let mut leaf_command: &Command = command;
        for name in &given.path {
            match leaf_command.find_subcommand(name) {
                Some(subcommand) => leaf_command = subcommand,
                None => return Step::Refuse,
            }
        }
        if leaf_command.has_subcommands() {
            return Step::Refuse;
        }
        let leaf = guide
            .get_leaves()
            .iter()
            .find(|leaf| leaf.path().iter().eq(given.path.iter()));
        let missing = missing(leaf_command, &given.ids, given.positionals);
        let Some(&arg) = missing.first() else {
            return Step::Refuse;
        };
        let id = arg.get_id().as_str();
        if missing.iter().any(|arg| !knows_values(arg, leaf))
            || self.asked.iter().any(|asked| asked == id)
        {
            return Step::Refuse;
        }
        self.asked.push(id.to_owned());
        let line = std::iter::once(command.get_name())
            .chain(given.path.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ");
        let arg = arg.clone();
        let id = arg.get_id().as_str();
        let choices = match choices(&arg, leaf) {
            Ok(choices) => choices,
            Err(error) => {
                let printing = Printing::of(guide);
                let failure = error.failure();
                failure::print(sink, json, spelling, printing, failure, Some(command));
                let exit = printing.exit(failure.code());
                #[cfg(feature = "usage-log")]
                crate::usage::exited(exit);
                return Step::Stop(exit);
            }
        };
        if choices.is_empty() {
            return Step::Refuse;
        }
        let help = arg.get_help().map(ToString::to_string);
        let name = arg
            .get_value_names()
            .and_then(|names| names.first())
            .map_or_else(|| id.to_uppercase(), ToString::to_string);
        let (least, most) = count(&arg);
        let mut question = Question::new(&line, &name, &choices).count(least, most);
        if let Some(help) = &help {
            question = question.help(help);
        }
        let answer = if self.standard {
            ask(self.picker, &mut Sink::standard_with_input(), &question)
        } else {
            ask(self.picker, sink, &question)
        };
        let Some(positions) = answer else {
            return Step::Refuse;
        };
        let values: Vec<&str> = positions
            .iter()
            .filter_map(|&position| choices.get(position).map(Choice::value))
            .collect();
        if values.len() != positions.len() || values.len() < least || values.len() > most {
            return Step::Refuse;
        }
        add(words, &arg, &values);
        Step::Picked
    }
}

/// Returns the guide's picker, or `None` when it has none or turns terminal
/// detection off, so a picker never runs without detection (ADR-0049).
pub(super) fn picker(guide: &Guide) -> Option<&'static dyn Picker> {
    guide.get_picker().filter(|_| guide.terminal_detection())
}

/// Reports whether clap refused a command line with `error` because it
/// leaves out a required argument, which a picker can fill in.
pub(super) fn wanted(error: &clap::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::MissingRequiredArgument | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

/// Reports whether the caller passed `--no-input`, in `matches` or in the
/// matches of any subcommand below it.
pub(super) fn no_input(matches: &ArgMatches) -> bool {
    let mut matches = matches;
    loop {
        if matches!(matches.try_get_one::<bool>(NO_INPUT_ID), Ok(Some(true))) {
            return true;
        }
        match matches.subcommand() {
            Some((_, subcommand)) => matches = subcommand,
            None => return false,
        }
    }
}

/// Returns the words that clap parses again after a pick: the root's name
/// and `words`. clap took the program name from the first parse.
pub(super) fn args(command: &Command, words: &[OsString]) -> Vec<OsString> {
    std::iter::once(OsString::from(command.get_name()))
        .chain(words.iter().cloned())
        .collect()
}

/// Prints the whole command line of a run whose missing arguments were
/// picked on the sink's error writer, such as `Running: shelf books show
/// 3`, so the person sees what runs and can type it next time.
pub(super) fn echo<O: Write, E: Write, R>(
    sink: &mut Sink<O, E, R>,
    command: &Command,
    words: &[OsString],
) {
    let mut line = format!("Running: {}", quote(command.get_name()));
    for word in words {
        line.push(' ');
        line.push_str(&quote(&word.to_string_lossy()));
    }
    // A run whose echo cannot be written still runs.
    let _ = writeln!(sink.err, "{line}");
}

/// Asks `picker` about `question` through the input and the error writer
/// of `sink`, and returns the picked positions, or `None` when no person
/// can answer, the person cancelled, or reading or writing failed.
fn ask<O, E: Write, R: BufRead>(
    picker: &dyn Picker,
    sink: &mut Sink<O, E, R>,
    question: &Question<'_>,
) -> Option<Vec<usize>> {
    if !sink.interactive {
        return None;
    }
    match picker.pick(question, &mut sink.input, &mut sink.err) {
        Ok(Answer::Picked(positions)) => Some(positions),
        Ok(_) | Err(_) => None,
    }
}

/// Returns the required arguments of `command` that the words leave out:
/// each required option whose id is not in `ids`, and each required
/// positional argument after the first `positionals` of them.
fn missing<'a>(command: &'a Command, ids: &[String], positionals: usize) -> Vec<&'a Arg> {
    let mut position = 0;
    let mut missing = Vec::new();
    for arg in command.get_arguments() {
        let given = if arg.is_positional() {
            position += 1;
            position <= positionals
        } else {
            ids.iter().any(|id| id == arg.get_id().as_str())
        };
        if arg.is_required_set() && !given {
            missing.push(arg);
        }
    }
    missing
}

/// Reports whether a picker can offer values for `arg`: it has possible
/// values, or `leaf` names a source for it.
fn knows_values(arg: &Arg, leaf: Option<&Leaf>) -> bool {
    !arg.get_possible_values().is_empty()
        || leaf.is_some_and(|leaf| {
            leaf.get_choices()
                .iter()
                .any(|choices| choices.arg() == arg.get_id().as_str())
        })
}

/// Returns the values that a picker offers for `arg`: its visible possible
/// values, or else the values of the source that `leaf` names for it.
///
/// # Errors
///
/// Returns the source's failure.
fn choices(arg: &Arg, leaf: Option<&Leaf>) -> Result<Vec<Choice>, crate::Error> {
    let possible = arg.get_possible_values();
    if !possible.is_empty() {
        return Ok(possible
            .iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| {
                let label = match value.get_help() {
                    Some(help) => format!("{}  {help}", value.get_name()),
                    None => value.get_name().to_owned(),
                };
                Choice::new(value.get_name().to_owned(), label)
            })
            .collect());
    }
    let source = leaf.and_then(|leaf| {
        leaf.get_choices()
            .iter()
            .find(|choices| choices.arg() == arg.get_id().as_str())
    });
    source.map_or_else(|| Ok(Vec::new()), Choices::load)
}

/// Returns the fewest and the most values that a person picks for `arg`.
/// An argument that clap appends to takes any number of values.
fn count(arg: &Arg) -> (usize, usize) {
    let range = arg.get_num_args();
    let least = range.map_or(1, |range| range.min_values().max(1));
    let most = if matches!(arg.get_action(), ArgAction::Append) {
        usize::MAX
    } else {
        range.map_or(1, |range| range.max_values())
    };
    (least, most)
}

/// Adds the picked `values` of `arg` to `words`.
///
/// An option's values go before the first `--`, or at the end when there
/// is none, each as `--long=value` or `-s=value`, so a value that starts
/// with `-` stays a value. A positional argument's values go at the end,
/// after a `--` that is added when a value starts with `-` and the words
/// have none.
//
// RUST-10: an option that takes several values in one occurrence and has no
// value delimiter gets one `--long=value` word for each value, which clap
// refuses as a repeated option. Writing the flag once followed by its values
// replaces this if a tool picks values for such an option.
fn add(words: &mut Vec<OsString>, arg: &Arg, values: &[&str]) {
    let end = words.iter().position(|word| word == "--");
    if arg.is_positional() {
        if end.is_none() && values.iter().any(|value| value.starts_with('-')) {
            words.push(OsString::from("--"));
        }
        words.extend(values.iter().map(OsString::from));
        return;
    }
    let flag = match (arg.get_long(), arg.get_short()) {
        (Some(long), _) => format!("--{long}="),
        (None, Some(short)) => format!("-{short}="),
        (None, None) => return,
    };
    let at = end.unwrap_or(words.len());
    let added: Vec<OsString> = match arg.get_value_delimiter() {
        Some(delimiter) if values.len() > 1 && !matches!(arg.get_action(), ArgAction::Append) => {
            vec![OsString::from(format!(
                "{flag}{}",
                values.join(&delimiter.to_string())
            ))]
        }
        _ => values
            .iter()
            .map(|value| OsString::from(format!("{flag}{value}")))
            .collect(),
    };
    words.splice(at..at, added);
}
