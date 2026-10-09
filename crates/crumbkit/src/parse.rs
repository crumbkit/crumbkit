//! `parse`, which parses a tool's command line with crumbkit's help lines,
//! and prints help, the version or a refusal in crumbkit's formats.

use std::ffi::OsString;
use std::io::{self, IsTerminal as _, Write};
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{Command, FromArgMatches};

use crate::refusal::Refusal;
use crate::{Sink, Spelling};

#[cfg(test)]
mod tests;

/// Parses the process's command line against `command`, and returns the
/// tool's parsed type and the command that it parsed against.
///
/// A tool's `main` calls it in place of clap's `Cli::parse`, and passes the
/// returned command to [`Output::set_command`](crate::Output::set_command):
///
/// ```no_run
/// use std::process::ExitCode;
///
/// use clap::{CommandFactory, Parser};
/// use crumbkit::{Output, Spelling};
///
/// /// List the books in a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {
///     #[command(flatten)]
///     output: Output,
/// }
///
/// fn main() -> ExitCode {
///     let (mut cli, command) = match crumbkit::parse::<Cli>(Cli::command(), Spelling::COLONS) {
///         Ok(parsed) => parsed,
///         Err(code) => return code,
///     };
///     cli.output.set_command(command);
///     // Run the command, then return `crumbkit::finish(...)`.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// It behaves as [`parse_to`] does with a sink over the process's standard
/// output and standard error, except that help and the version keep clap's
/// colours when standard output is a terminal. clap prints those colours only
/// when the tool enables clap's `color` feature.
///
/// # Errors
///
/// Returns the exit code for `main` to return after crumbkit has printed
/// help, the version or a refusal.
pub fn parse<C: FromArgMatches>(
    command: Command,
    spelling: Spelling,
) -> Result<(C, Command), ExitCode> {
    parse_from(command, spelling, std::env::args_os())
}

/// Parses `args` against `command` as [`parse`] parses the process's command
/// line. The first argument is the program name, as in clap's `parse_from`.
///
/// # Errors
///
/// Returns the exit code for `main` to return after crumbkit has printed
/// help, the version or a refusal.
pub fn parse_from<C, I, T>(
    command: Command,
    spelling: Spelling,
    args: I,
) -> Result<(C, Command), ExitCode>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut sink = Sink::standard();
    let colour = sink.out.is_terminal();
    run(&mut sink, command, spelling, args, colour)
}

/// Parses `args` against `command` and prints help, the version or a
/// refusal to `sink`, and returns what [`parse_from`] would return.
///
/// Tests and golden files call it with a sink over in-memory writers, as
/// they call [`finish_to`](crate::finish_to). Help and the version print as
/// plain text.
///
/// Before parsing, crumbkit walks the typed words down the tree to the
/// deepest command they name, and adds its lines to the help of each command
/// on the way, the root included:
///
/// - `-h` ends with a line that names the root's visible global flags, such
///   as ``Global flags: --json, --fields <FIELD>, --long, --full. `shelf
///   books list --help` describes them.`` crumbkit hides those flags from
///   `-h`, so `-h` stays short and `--help` lists them in full.
/// - The help of a command with subcommands ends with a line that opens one
///   of them, such as `Next: shelf books <COMMAND> -h`, in `-h` and in
///   `--help`.
///
/// Help and the version print to the sink's output writer, and the exit code
/// is 0, also when the reader closed the pipe early.
///
/// A command line that clap refuses prints as a failure with the code
/// `crumbkit::cli::usage` in `spelling`, and the exit code is 2. The message
/// is clap's error on one line, and the recovery runs the `-h` of the
/// deepest command:
///
/// ```text
/// error: unexpected argument '--id' found; a similar argument exists: '--ids'
///   help:  run `shelf books show -h`
///   code:  crumbkit::cli::usage
/// ```
///
/// A command line that stops at a command with subcommands, such as `shelf
/// books`, prints the message ``the command `shelf books` needs a
/// subcommand``. With `--json` before any `--`, the refusal prints to the
/// output writer as one `crumbkit/failure/v1` document, as a failure does
/// with `finish`.
///
/// # Errors
///
/// Returns the exit code for `main` to return after crumbkit has printed
/// help, the version or a refusal.
pub fn parse_to<C, I, T, O, E>(
    sink: &mut Sink<O, E>,
    command: Command,
    spelling: Spelling,
    args: I,
) -> Result<(C, Command), ExitCode>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    O: Write,
    E: Write,
{
    run(sink, command, spelling, args, false)
}

/// Parses `args` against `command` as [`parse_to`] describes. With `colour`,
/// help and the version print through clap, which writes them to standard
/// output with the colours that the tool's clap features allow.
fn run<C, I, T, O, E>(
    sink: &mut Sink<O, E>,
    mut command: Command,
    spelling: Spelling,
    args: I,
    colour: bool,
) -> Result<(C, Command), ExitCode>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    O: Write,
    E: Write,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    // No `Output` exists before parsing, so crumbkit looks for `--json` in
    // the words before any `--`.
    let json = args
        .iter()
        .skip(1)
        .take_while(|word| *word != "--")
        .any(|word| word == "--json");
    let path = walk(&command, &args);
    add_help_lines(&mut command, &path);

    let error = match command.try_get_matches_from_mut(args) {
        Ok(mut matches) => match C::from_arg_matches_mut(&mut matches) {
            Ok(parsed) => return Ok((parsed, command)),
            Err(error) => error,
        },
        Err(error) => error,
    };
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        return Err(print_page(sink, &error, colour));
    }
    let refusal = Refusal::command_line(refusal_message(&command, &path, &error), path);
    Err(refusal.print(sink, json, spelling, Some(&command)))
}

/// Returns the real names of the subcommands from the root to the deepest
/// command that `args` names, after the program name.
///
/// It skips the program name, stops at `--`, skips each word that starts
/// with `-`, and descends into each word that names a subcommand of the
/// current command. `Command::find_subcommand` resolves an alias to its
/// command and finds a hidden command, because the caller typed its name.
//
// RUST-10: the walk reads each word on its own, as a subcommand name or a
// flag. It misreads an option's value in its own word, or a positional, that
// equals a subcommand name, as in `shelf --fields books`, and descends into
// that command. It runs before parsing because help needs its lines
// before clap prints it. A walk that reads how many values each option
// takes from the tree, as clap's parser does, replaces it.
fn walk(command: &Command, args: &[OsString]) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = command;
    for word in args.iter().skip(1) {
        if word == "--" {
            break;
        }
        if word.as_encoded_bytes().starts_with(b"-") {
            continue;
        }
        if let Some(subcommand) = current.find_subcommand(word) {
            path.push(subcommand.get_name().to_owned());
            current = subcommand;
        }
    }
    path
}

/// Adds crumbkit's help lines to the root and to each command on `path`.
///
/// It hides the root's visible global flags from `-h` first, so the copies
/// that clap gives each subcommand are hidden too. It then builds the root
/// alone with `Command::render_usage`, because `Arg`'s `Display` reads an
/// option's number of values, which clap sets in the build. The parse that
/// follows builds only the commands it reaches, and `Command::build` would
/// build the whole tree.
fn add_help_lines(command: &mut Command, path: &[String]) {
    *command = std::mem::take(command).mut_args(|arg| {
        if arg.is_global_set() && !arg.is_hide_set() {
            arg.hide_short_help(true)
        } else {
            arg
        }
    });
    let _ = command.render_usage();
    let flags: Vec<String> = command
        .get_arguments()
        .filter(|arg| arg.is_global_set() && !arg.is_hide_set())
        .map(ToString::to_string)
        .collect();
    let flags = flags.join(", ");

    let mut text = command.get_name().to_owned();
    let mut node = command;
    let mut names = path.iter();
    loop {
        set_help_lines(node, &text, &flags);
        let Some(name) = names.next() else {
            break;
        };
        let Some(subcommand) = node.find_subcommand_mut(name) else {
            break;
        };
        text.push(' ');
        text.push_str(name);
        node = subcommand;
    }
}

/// Sets the lines that end the help of `node`, whose path is `text`: the
/// global flags line in `-h` when `flags` is not empty, and the `Next:` line
/// in `-h` and `--help` when `node` has subcommands.
fn set_help_lines(node: &mut Command, text: &str, flags: &str) {
    let mut short = Vec::new();
    let mut long = Vec::new();
    if !flags.is_empty() {
        short.push(format!(
            "Global flags: {flags}. `{text} --help` describes them."
        ));
    }
    if node.has_subcommands() {
        let value_name = node.get_subcommand_value_name().unwrap_or("COMMAND");
        let next = format!("Next: {text} <{value_name}> -h");
        short.push(next.clone());
        long.push(next);
    }
    if short.is_empty() {
        return;
    }
    // `--help` falls back to `after_help` when `after_long_help` is unset, so
    // a command without a `Next:` line gets an empty `after_long_help`, and
    // clap trims the blank lines it leaves.
    *node = std::mem::take(node)
        .after_help(short.join("\n"))
        .after_long_help(long.join("\n"));
}

/// Prints the help page or the version in `error` and returns the exit code:
/// 0 when it is written or the reader closed the pipe early, and 1 when the
/// write fails for another reason. With `colour`, clap's `Error::print`
/// writes it to standard output with clap's colours, and otherwise it goes
/// to the sink's output writer as plain text.
fn print_page<O: Write, E: Write>(
    sink: &mut Sink<O, E>,
    error: &clap::Error,
    colour: bool,
) -> ExitCode {
    let written = if colour {
        error.print()
    } else {
        write!(sink.out, "{}", error.render()).and_then(|()| sink.out.flush())
    };
    match written {
        Ok(()) => ExitCode::SUCCESS,
        // A reader that stops early, as `shelf -h | head -1` does, closes the
        // pipe after it has read every line it wanted.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            // The process is already failing, and a failed report has nowhere
            // else to go.
            let _ = writeln!(sink.err, "error: cannot write to standard output: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Returns the message of a refusal of a command line that ends at the
/// command at `path`.
///
/// A command line that stops at a command that needs more, which clap
/// answers with that command's help page or a list of its subcommands, says
/// what the command needs. Every other refusal is clap's own message, from
/// [`message`].
fn refusal_message(command: &Command, path: &[String], error: &clap::Error) -> String {
    if !matches!(
        error.kind(),
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand | ErrorKind::MissingSubcommand
    ) {
        return message(&error.render().to_string());
    }
    let mut text = command.get_name().to_owned();
    let mut node = command;
    for name in path {
        text.push(' ');
        text.push_str(name);
        if let Some(subcommand) = node.find_subcommand(name) {
            node = subcommand;
        }
    }
    if node.has_subcommands() {
        format!("the command `{text}` needs a subcommand")
    } else {
        format!("the command `{text}` needs arguments")
    }
}

/// Returns clap's rendered error text on one line.
///
/// It keeps the text above the `Usage:` line and the `For more information`
/// line, and drops the `error:` and `tip:` labels. The lines of a paragraph
/// join with spaces, and paragraphs and tips join with `; `, as in
/// `unexpected argument '--id' found; a similar argument exists: '--ids'`.
/// Without clap's `error-context` feature, clap's text is one line, such as
/// `error: unexpected argument found`, and the message is that line.
//
// RUST-10: this reads clap's wording and layout, which clap does not
// promise to keep. It misses any part that clap adds below the usage line.
// shelf's golden files fail when the wording changes. A message built from
// the error's kind and its context values, through `Error::context`,
// replaces it once a change makes the text unreadable.
fn message(rendered: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut new_part = true;
    for line in rendered.lines() {
        let line = line.trim();
        if line.starts_with("Usage:") || line.starts_with("For more information") {
            break;
        }
        if line.is_empty() {
            new_part = true;
            continue;
        }
        let (line, labelled) = match line
            .strip_prefix("error:")
            .or_else(|| line.strip_prefix("tip:"))
        {
            Some(rest) => (rest.trim_start(), true),
            None => (line, false),
        };
        match parts.last_mut() {
            Some(part) if !new_part && !labelled => {
                part.push(' ');
                part.push_str(line);
            }
            _ => parts.push(line.to_owned()),
        }
        new_part = false;
    }
    parts.join("; ")
}
