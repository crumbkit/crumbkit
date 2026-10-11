//! `parse`, which parses a tool's command line with crumbkit's help lines,
//! and prints help, the version or a refusal in crumbkit's formats.

use std::ffi::OsString;
use std::fmt::Display;
use std::io::{self, IsTerminal as _, Write};
use std::process::ExitCode;

use clap::error::ErrorKind;
use clap::{Command, FromArgMatches};

use self::lines::{add_help_lines, has_visible_subcommands};
use self::walk::{help_word, walk};
use crate::help::{Guide, Topic, topic};
use crate::refusal::Refusal;
use crate::{Sink, Spelling};

mod lines;
pub(crate) mod suggestion;
#[cfg(test)]
mod tests;
mod walk;

/// Parses the process's command line against `command` with the tool's
/// `guide`, and returns the tool's parsed type and the command that it
/// parsed against.
///
/// A tool's `main` calls it in place of clap's `Cli::parse`, and passes the
/// returned command to [`Output::set_command`](crate::Output::set_command).
/// A tool without goal headings or topics passes `&Guide::new()`:
///
/// ```no_run
/// use std::process::ExitCode;
///
/// use clap::{CommandFactory, Parser};
/// use crumbkit::help::Guide;
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
///     let parsed = crumbkit::parse::<Cli>(Cli::command(), &Guide::new(), Spelling::COLONS);
///     let (mut cli, command) = match parsed {
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
    guide: &Guide,
    spelling: Spelling,
) -> Result<(C, Command), ExitCode> {
    parse_from(command, guide, spelling, std::env::args_os())
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
    guide: &Guide,
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
    run(&mut sink, command, guide, spelling, args, colour)
}

/// Parses `args` against `command` and prints help, the version or a
/// refusal to `sink`, and returns what [`parse_from`] would return.
///
/// Tests and golden files call it with a sink over in-memory writers, as
/// they call [`finish_to`](crate::finish_to). Help and the version print as
/// plain text.
///
/// Before parsing, crumbkit walks the typed words down the tree to the
/// deepest command they name, and changes the help of each command on the
/// way, the root included. The walk reads each option's values as clap's
/// parser does, from the option's `Arg`, so an option's value that equals a
/// subcommand name, as in `shelf --fields books`, names no command.
///
/// A command that `guide` gives goals lists its subcommands under their
/// headings, as [`help::extend`](crate::help::extend) describes. The list
/// replaces the command's own `before_help` and `before_long_help`, so
/// neither text prints. crumbkit adds its lines after the command's own
/// `after_help` and `after_long_help`:
///
/// - `-h` ends with a line that names the visible global flags of the root
///   and of each command on the way, the root's first, such as ``Global
///   flags: --json, --fields <FIELD>, --long, --full. `shelf books list
///   --help` describes them.`` crumbkit hides those flags from `-h`, so `-h`
///   stays short and `--help` lists them in full.
/// - The root's help names the guide's topics on one line, such as
///   ``Topics: catalogue. `shelf help <TOPIC>` prints one.``, in `-h` and in
///   `--help`.
/// - The help of a command with a visible subcommand ends with a line that
///   opens one of them, such as `Next: shelf books <COMMAND> -h`, in `-h`
///   and in `--help`.
///
/// When the deepest command typed has a [`Leaf`](crate::help::Leaf) in
/// `guide`, its help gains the leaf's sections after the options and before
/// crumbkit's lines:
///
/// - `-h` and `--help` list the leaf's examples under `Examples:`, each as
///   the full command line, such as `shelf books list --all`. A word that a
///   POSIX shell would split or expand prints in single quotes. `--help`
///   prints each example's description on the next line, in the layout of
///   clap's long options.
/// - `--help` lists the row type's columns under `Columns:` and the columns
///   that `--long` adds under `Columns that --long adds:`, each with the
///   first paragraph of its field's doc comment, which crumbkit reads from
///   the row type's JSON Schema. crumbkit builds the schema only when a
///   word before any `--` reads as the flag `--help`, so a run that prints
///   no long help never builds it.
/// - `-h` of a leaf with a document ends with a line such as
///   `Next: shelf books list --help explains each column.`
///
/// A leaf off the typed path, and a leaf whose path names no command, leave
/// every help page unchanged.
///
/// crumbkit answers the word `help` in place of clap, whose generated
/// `help` subcommand it turns off. It answers `help` as the next word below
/// any command with subcommands, and directly after the program name when
/// the guide has topics:
///
/// - `shelf help` lists the tool's topics under the heading `Topics` and
///   ends with `Next: shelf help <TOPIC>`. A tool without topics prints the
///   root's `--help`.
/// - `shelf help catalogue` prints the text of the topic `catalogue`. A root
///   command or alias with that name wins over the topic. Topics belong to
///   the root, so `shelf books help catalogue` names no command. A root with
///   its own command named or aliased `help` has no topics.
/// - Any other `help` command line drops the word `help` and asks for
///   `--help`. `shelf help books show` and `shelf books help show` print
///   what `shelf books show --help` prints, `shelf books help` prints what
///   `shelf books --help` prints, and clap refuses a word that names no
///   command, as in `shelf help comets` or `shelf books help shwo`.
///
/// A command without subcommands reads `help` as an ordinary word, such as
/// the value of a positional argument. A command with its own subcommand
/// named or aliased `help` lets clap parse that subcommand.
///
/// Help, the version, a topic and the topic list print to the sink's output
/// writer, and the exit code is 0, also when the reader closed the pipe
/// early.
///
/// A command line that clap refuses prints as a failure with the code
/// `crumbkit::cli::usage` in `spelling`, and the exit code is 2. The message
/// is clap's error on one line, and the recovery runs the `-h` of the
/// deepest command, as shelf's golden file
/// [`books-show-2-comma-5.stderr`][books-show-2-comma-5] shows.
///
/// clap suggests from every name that a flag or a subcommand accepts. When
/// it suggests a hidden alias, such as `--max` of `--limit`, the message
/// names the real flag or subcommand instead, once. The message leaves out a
/// suggested flag or subcommand that the tool hid, and leaves out clap's tip
/// when it suggests nothing else.
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
///
/// [books-show-2-comma-5]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-2-comma-5.stderr
pub fn parse_to<C, I, T, O, E>(
    sink: &mut Sink<O, E>,
    command: Command,
    guide: &Guide,
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
    run(sink, command, guide, spelling, args, false)
}

/// Parses `args` against `command` as [`parse_to`] describes. With the
/// `tracing` feature, parsing runs inside the `crumbkit.parse` span, which
/// records the exit code when parsing ends the run (ADR-0030).
fn run<C, I, T, O, E>(
    sink: &mut Sink<O, E>,
    command: Command,
    guide: &Guide,
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
    #[cfg(feature = "tracing")]
    let span = crate::trace::spans::parse();
    #[cfg(feature = "tracing")]
    let _entered = span.enter();
    let parsed = read(sink, command, guide, spelling, args, colour);
    #[cfg(feature = "tracing")]
    if let Err(code) = parsed {
        crate::trace::spans::exited(&span, code);
    }
    parsed
}

/// Parses `args` against `command` as [`parse_to`] describes. With `colour`,
/// help and the version print through clap, which writes them to standard
/// output with the colours that the tool's clap features allow.
fn read<C, I, T, O, E>(
    sink: &mut Sink<O, E>,
    mut command: Command,
    guide: &Guide,
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
    let mut args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    #[cfg(feature = "usage-log")]
    crate::usage::typed(&command, &args);
    // crumbkit answers `help` itself. clap propagates the setting to every
    // subcommand, so no `Commands:` list shows clap's `help` line, and
    // `Command::build` copies no subtree into a generated help tree.
    command = command.disable_help_subcommand(true);
    // A root command named or aliased `help` takes the word, so no topic
    // can print.
    let topics = if command.find_subcommand("help").is_some() {
        &[]
    } else {
        guide.get_topics()
    };
    // No `Output` exists before parsing, so crumbkit looks for `--json` in
    // the words before any `--`.
    let json = args
        .iter()
        .skip(1)
        .take_while(|word| *word != "--")
        .any(|word| word == "--json");
    if let Some(help) = help_word(&command, !topics.is_empty(), &args) {
        if help.at_root
            && let Some(page) = help_page(&command, topics, &args[help.position + 1..])
        {
            // RUST-10: a topic and the topic list print as plain text, also
            // on a terminal, because `StyledStr`'s `Display` drops styles. A
            // page that keeps the styles, written as clap's `Error::print`
            // writes help, replaces this when a tool needs colour there.
            let code = print_text(sink, &page);
            #[cfg(feature = "usage-log")]
            crate::usage::helped(&command, &[], true, code);
            return Err(code);
        }
        ask_for_help(&mut args, help.position);
    }
    // A leaf's `--help` describes its columns from the row type's JSON
    // Schema, so crumbkit builds the schema only when the walk reads
    // `--help` as a flag, also after the `help` rewrite above.
    let walked = walk(&command, &args);
    let (path, long_help) = (walked.path, walked.long_help);
    #[cfg(feature = "tracing")]
    crate::trace::spans::command(&path);
    add_help_lines(&mut command, guide, topics, &path, long_help);

    let error = match command.try_get_matches_from_mut(args) {
        Ok(mut matches) => {
            #[cfg(feature = "usage-log")]
            crate::usage::parsed(&command, &matches);
            match C::from_arg_matches_mut(&mut matches) {
                Ok(parsed) => return Ok((parsed, command)),
                Err(error) => error,
            }
        }
        Err(error) => error,
    };
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        let code = print_page(sink, &error, colour);
        #[cfg(feature = "usage-log")]
        if error.kind() == ErrorKind::DisplayHelp {
            crate::usage::helped(&command, &path, long_help, code);
        } else {
            crate::usage::exited(code);
        }
        return Err(code);
    }
    #[cfg(feature = "usage-log")]
    crate::usage::refused(&command, &path, &error);
    let refusal = Refusal::command_line(refusal_message(&command, &path, &error), path);
    Err(refusal.print(sink, json, spelling, Some(&mut command)))
}

/// Returns the page that `help` at the root prints without clap, for
/// `rest`, the words after `help`: the list of `topics` when `rest` holds no
/// word that names something, and a topic's text when its one such word
/// names a topic and no root command or alias has that name. It returns
/// `None` otherwise, and the caller asks clap for help instead.
///
/// The words that name something are the words before any `--` that do not
/// start with `-`.
//
// RUST-10: the page ignores flags, so `shelf help catalogue --json` prints
// the topic as text. A topic in JSON replaces this when a caller needs it.
fn help_page(command: &Command, topics: &[Topic], rest: &[OsString]) -> Option<String> {
    let mut words = rest
        .iter()
        .take_while(|word| *word != "--")
        .filter(|word| !word.as_encoded_bytes().starts_with(b"-"));
    match (words.next(), words.next()) {
        (None, _) => topic::list(command.get_name(), topics),
        (Some(word), None) if command.find_subcommand(word).is_none() => {
            topic::text(topics, word.to_str()?)
        }
        _ => None,
    }
}

/// Turns `help` at `position` into a request for clap's long help: it
/// removes the word and adds `--help` before the first `--`, or at the end
/// when there is none. `shelf help books show` and `shelf books help show`
/// then print what `shelf books show --help` prints, and clap refuses a
/// word that names no command, as in `shelf help comets`.
//
// RUST-10: the request needs the tool's `--help` flag. A tool that disables
// it gets clap's refusal of `--help`. crumbkit's own help flag replaces the
// request if a tool needs that.
fn ask_for_help(args: &mut Vec<OsString>, position: usize) {
    args.remove(position);
    let end = args
        .iter()
        .position(|word| word == "--")
        .unwrap_or(args.len());
    args.insert(end, OsString::from("--help"));
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
    if colour {
        let written = error.print();
        exit_code(sink, written)
    } else {
        print_text(sink, &error.render())
    }
}

/// Prints `text` to the sink's output writer as plain text and returns the
/// exit code, as [`print_page`] does.
fn print_text<O: Write, E: Write>(sink: &mut Sink<O, E>, text: &dyn Display) -> ExitCode {
    let written = write!(sink.out, "{text}").and_then(|()| sink.out.flush());
    exit_code(sink, written)
}

/// Returns the exit code after a page was `written`, and reports a failed
/// write on the sink's error writer.
fn exit_code<O: Write, E: Write>(sink: &mut Sink<O, E>, written: io::Result<()>) -> ExitCode {
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
/// [`message`], with the real name in place of each name that clap
/// suggests and without the names of hidden items, from
/// [`suggestion::real_names`].
fn refusal_message(command: &Command, path: &[String], error: &clap::Error) -> String {
    let mut text = command.get_name().to_owned();
    let mut commands = vec![command];
    for name in path {
        text.push(' ');
        text.push_str(name);
        if let Some(subcommand) = commands.last().and_then(|node| node.find_subcommand(name)) {
            commands.push(subcommand);
        }
    }
    if !matches!(
        error.kind(),
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand | ErrorKind::MissingSubcommand
    ) {
        // `message` joins the parts with `; `, and `real_names` changes or
        // drops only a part that starts with clap's suggestion, so splitting
        // on `; ` and joining again keeps every other part as it was.
        return message(&error.render().to_string())
            .split("; ")
            .filter_map(|part| suggestion::real_names(part, &commands))
            .collect::<Vec<_>>()
            .join("; ");
    }
    let node = commands.last().copied().unwrap_or(command);
    if has_visible_subcommands(node) {
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
