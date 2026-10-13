//! `parse`, which parses a tool's command line with crumbkit's help lines,
//! and prints help, the version or a refusal in crumbkit's formats.

use std::ffi::OsString;
use std::io::{BufRead, IsTerminal as _, Write};

use clap::error::ErrorKind;
use clap::{Command, FromArgMatches};

pub use self::parsed::Parsed;
pub use self::stop::Stop;

use self::attempt::attempt;
use self::lines::{add_help_lines, has_visible_subcommands};
use self::page::{print_page, print_text};
use self::pick::{Picking, Step};
use self::walk::{help_word, program_stem, walk};
use crate::help::{Guide, topic};
use crate::output::failure::Printing;
use crate::refusal::Refusal;
use crate::{Sink, Spelling};

mod attempt;
mod declared;
mod lines;
mod page;
mod parsed;
mod pick;
mod stop;
pub(crate) mod suggestion;
#[cfg(test)]
mod tests;
pub(crate) mod walk;

/// Parses the process's command line against `command` with the tool's
/// `guide`, and returns the tool's parsed type and a [`Parsed`], which holds
/// the command that it parsed against and the words that the caller typed.
///
/// A tool's `main` calls it in place of clap's `Cli::parse`, and passes the
/// returned `Parsed` to [`Output::set_command`](crate::Output::set_command).
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
///     let (mut cli, parsed) = match parsed {
///         Ok(parsed) => parsed,
///         Err(stop) => return stop.code(),
///     };
///     cli.output.set_command(parsed);
///     // Run the command, then return `crumbkit::finish(...)`.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// It behaves as [`parse_to`] does with a sink over the process's standard
/// output and standard error, except that help and the version keep clap's
/// colours when standard output is a terminal and the guide detects terminals.
/// clap prints those colours only when the tool enables clap's `color` feature.
///
/// # Errors
///
/// Returns a [`Stop`] after crumbkit has printed help, the version or a
/// refusal. It holds the exit code for `main` to return, and whether the
/// caller asked for JSON.
pub fn parse<C: FromArgMatches>(
    command: Command,
    guide: &Guide,
    spelling: Spelling,
) -> Result<(C, Parsed), Stop> {
    parse_from(command, guide, spelling, std::env::args_os())
}

/// Parses `args` against `command` as [`parse`] parses the process's command
/// line. The first argument is the program name, as in clap's `parse_from`.
///
/// # Errors
///
/// Returns a [`Stop`] after crumbkit has printed help, the version or a
/// refusal. It holds the exit code for `main` to return, and whether the
/// caller asked for JSON.
pub fn parse_from<C, I, T>(
    command: Command,
    guide: &Guide,
    spelling: Spelling,
    args: I,
) -> Result<(C, Parsed), Stop>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut sink = Sink::standard();
    let colour = guide.detects(|| sink.out.is_terminal());
    run(&mut sink, command, guide, spelling, args, colour, true)
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
/// headings, as [`help::extend`](crate::help::extend) describes, below the
/// command's own `before_help` or `before_long_help`. crumbkit adds its
/// lines after the command's own `after_help` and `after_long_help`:
///
/// - `-h` ends with a line that names the visible global flags of the root
///   and of each command on the way, the root's first, such as ``Global
///   flags: --json, --fields <FIELD>, --long, --full. `shelf books list
///   --help` describes them.`` crumbkit hides those flags from `-h`, so `-h`
///   stays short and `--help` lists them in full.
/// - The root's help names the guide's topics and crumbkit's `exit-codes`
///   topic on one line, such as ``Topics: catalogue, exit-codes. `shelf
///   help <TOPIC>` prints one.``, in `-h` and in `--help`. A root without
///   subcommands and without topics of the tool's own has no such line.
/// - When `guide` has [`Guide::personal_aliases`], the clap root's `-h` and
///   `--help` name the person's aliases on one line, such as ``Personal
///   aliases: firsts, mine. `shelf aliases list` shows what each runs.``
///   Only a run that prints the clap root's help calls the guide's
///   function.
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
/// - `-h` of a leaf whose document has columns ends with a line such as
///   `Next: shelf books list --help explains each column.` A prose
///   document has no columns, so its leaf has neither the line nor the
///   column sections.
/// - `-h` and `--help` of a leaf that declares its
///   [`Effects`](crate::help::Effects) print them on one line before
///   crumbkit's lines, such as `Effects: destructive, idempotent,
///   closed-world.` A leaf without effects prints no such line.
///
/// A leaf off the typed path, and a leaf whose path names no command, leave
/// every help page unchanged.
///
/// crumbkit answers the word `help` in place of clap, whose generated
/// `help` subcommand it turns off. It answers `help` as the next word below
/// any command with subcommands, and directly after the program name when
/// the root has subcommands or the guide has topics:
///
/// - `shelf help` lists the tool's topics and then `exit-codes` under the
///   heading `Topics`, and ends with `Next: shelf help <TOPIC>`.
/// - `shelf help catalogue` prints the text of the topic `catalogue`. A root
///   command or alias with that name wins over the topic. Topics belong to
///   the root, so `shelf books help catalogue` names no command. A root with
///   its own command named or aliased `help` has no topics.
/// - `shelf help exit-codes` lists crumbkit's exit codes 0, 1 and 2 and
///   then the [`Exit`](crate::help::Exit) codes of the guide, each with its
///   meaning. It leaves out 2 when the guide lists another exit for a
///   refused command line. A topic of the tool's own named `exit-codes` never prints,
///   because crumbkit's topic has the name.
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
/// subcommand``. With `--json`, the refusal prints to the output writer as
/// one `crumbkit/failure/v1` document, as a failure does with `finish`. When
/// `guide` sets a [`FailureForm`](crate::form::FailureForm), the refusal
/// prints in that form in text and with `--json`. The
/// walk finds `--json` in the words before any `--` where clap reads it as a
/// flag, so a value of an option that allows hyphen values, as in
/// `--pattern --json`, is no request for JSON.
///
/// # Errors
///
/// Returns a [`Stop`] after crumbkit has printed help, the version or a
/// refusal. It holds the exit code for `main` to return, and whether the
/// caller asked for JSON.
///
/// [books-show-2-comma-5]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/tests/golden/books-show-2-comma-5.stderr
pub fn parse_to<C, I, T, O, E>(
    sink: &mut Sink<O, E, impl BufRead>,
    command: Command,
    guide: &Guide,
    spelling: Spelling,
    args: I,
) -> Result<(C, Parsed), Stop>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    O: Write,
    E: Write,
{
    run(sink, command, guide, spelling, args, false, false)
}

/// Parses `args` against `command` as [`parse_to`] describes. With the
/// `tracing` feature, parsing runs inside the `crumbkit.parse` span, which
/// records the exit code when parsing ends the run (ADR-0030).
fn run<C, I, T, O, E, R>(
    sink: &mut Sink<O, E, R>,
    command: Command,
    guide: &Guide,
    spelling: Spelling,
    args: I,
    colour: bool,
    stdin: bool,
) -> Result<(C, Parsed), Stop>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    O: Write,
    E: Write,
    R: BufRead,
{
    #[cfg(feature = "tracing")]
    let span = crate::trace::spans::parse();
    #[cfg(feature = "tracing")]
    let _entered = span.enter();
    let parsed = read(sink, command, guide, spelling, args, colour, stdin);
    #[cfg(feature = "tracing")]
    if let Err(stop) = &parsed {
        crate::trace::spans::exited(&span, stop.number());
    }
    parsed
}

/// Parses `args` against `command` as [`parse_to`] describes. With `colour`, help and the version print
/// through clap, which writes them to standard output with the colours that
/// the tool's clap features allow. With `stdin`, the guide's picker reads
/// the process's standard input and writes to its standard error, and
/// otherwise it uses the sink's input and error writer.
fn read<C, I, T, O, E, R>(
    sink: &mut Sink<O, E, R>,
    mut command: Command,
    guide: &Guide,
    spelling: Spelling,
    args: I,
    colour: bool,
    stdin: bool,
) -> Result<(C, Parsed), Stop>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    O: Write,
    E: Write,
    R: BufRead,
{
    let mut args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    if cfg!(windows) {
        program_stem(&mut args);
    }
    #[cfg(feature = "usage-log")]
    crate::usage::typed(&command, &args, spelling);
    // crumbkit answers `help` itself. clap propagates the setting to every
    // subcommand, so no `Commands:` list shows clap's `help` line, and
    // `Command::build` copies no subtree into a generated help tree.
    command = command.disable_help_subcommand(true);
    // The guide's root, which is the clap root unless `Guide::root` names
    // a subcommand, answers `help` with topics when it has subcommands or
    // the tool declares topics, and then it also has crumbkit's
    // `exit-codes` topic. A command named or aliased `help` below the root
    // takes the word, so no topic can print.
    let root = guide.root_in(&command);
    let has_topics = {
        let node = root.iter().fold(&command, |node, name| {
            node.find_subcommand(name).unwrap_or(node)
        });
        node.find_subcommand("help").is_none()
            && (node.has_subcommands() || !guide.get_topics().is_empty())
    };
    if let Some(help) = help_word(&command, root, has_topics, &args) {
        if help.at_root
            && let Some(page) = help_page(&command, guide, &args[help.position + 1..])
        {
            // RUST-10: a topic and the topic list print as plain text, also
            // on a terminal, because `StyledStr`'s `Display` drops styles. A
            // page that keeps the styles, written as clap's `Error::print`
            // writes help, replaces this when a tool needs colour there.
            let code = print_text(sink, &page);
            #[cfg(feature = "usage-log")]
            crate::usage::helped(
                &command,
                &root.iter().map(|&name| name.to_owned()).collect::<Vec<_>>(),
                true,
                code,
            );
            // Only this path ends before the walk below, so it walks the
            // words for `--json` here.
            let json = walk(&command, &args).json;
            return Err(Stop::new(crate::exit::number(code), json, command, guide));
        }
        ask_for_help(&mut args, help.position);
    }
    // A leaf's `--help` describes its columns from the row type's JSON
    // Schema, so crumbkit builds the schema only when the walk reads
    // `--help` as a flag, also after the `help` rewrite above.
    let walked = walk(&command, &args);
    // No `Output` exists before parsing, so a refusal and a `Stop` take
    // `--json` from the walk.
    let (path, long_help, json) = (walked.path, walked.long_help, walked.json);
    // Only a run that prints the clap root's help reads a person's aliases.
    let aliases = if walked.help && path.is_empty() {
        guide.get_personal_aliases()
    } else {
        None
    };
    #[cfg(feature = "tracing")]
    crate::trace::spans::command(&path);
    add_help_lines(&mut command, guide, has_topics, aliases, &path, long_help);

    // `Output::confirm` repeats the typed words, so they outlive parsing.
    let mut words = args.get(1..).unwrap_or_default().to_vec();
    let mut parsed = attempt::<C>(&mut command, args, guide);
    // A command line that leaves out a required argument can have it
    // picked, one argument at a time, and parses again after each pick.
    if let Some(picker) = pick::picker(guide) {
        let mut picking = Picking::new(picker, stdin);
        let mut picked = 0;
        while let Err(error) = &parsed
            && pick::wanted(error)
        {
            match picking.pick(sink, &mut command, guide, &mut words, json, spelling) {
                Step::Picked => picked += 1,
                Step::Refuse => break,
                Step::Stop(code) => return Err(Stop::new(code, json, command, guide)),
            }
            let args = pick::args(&command, &words);
            parsed = attempt::<C>(&mut command, args, guide);
        }
        if picked > 0 && parsed.is_ok() {
            pick::echo(sink, &command, &words);
            #[cfg(feature = "usage-log")]
            crate::usage::picked(picked);
        }
    }
    let error = match parsed {
        Ok((parsed, no_input, None)) => {
            return Ok((parsed, Parsed::new(command, words, no_input, guide)));
        }
        Ok((_, _, Some(refusal))) => {
            #[cfg(feature = "usage-log")]
            crate::usage::exited(Printing::of(guide).exit(&crate::codes::CLI_USAGE));
            return Err(refuse(sink, &refusal, json, spelling, guide, command));
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
            crate::usage::exited(crate::exit::number(code));
        }
        return Err(Stop::new(crate::exit::number(code), json, command, guide));
    }
    #[cfg(feature = "usage-log")]
    crate::usage::refused(
        &command,
        &path,
        &error,
        Printing::of(guide).exit(&crate::codes::CLI_USAGE),
    );
    let message = refusal_message(&command, &path, &error);
    let declared = declared::alternatives::<C>(&mut command, guide, &path, &error, &words);
    let refusal = Refusal::command_line(message, path, &error, &words, declared);
    Err(refuse(sink, &refusal, json, spelling, guide, command))
}

/// Prints `refusal` to `sink` in the form, the layout and with the code
/// that `guide` decides, and returns the `Stop` that ends the run with the
/// exit code that `guide` gives a refused command line.
fn refuse<O: Write, E: Write, R>(
    sink: &mut Sink<O, E, R>,
    refusal: &Refusal,
    json: bool,
    spelling: Spelling,
    guide: &Guide,
    mut command: Command,
) -> Stop {
    let code = refusal.print(
        sink,
        json,
        spelling,
        Printing::of(guide),
        Some(&mut command),
    );
    Stop::new(code, json, command, guide)
}

/// Returns the page that `help` at the guide's root prints without clap,
/// for `rest`, the words after `help`: the list of the topics of `guide`
/// and `exit-codes` when `rest` holds no word that names something, and a
/// topic's text when its one such word names a topic and no command or
/// alias directly below the root has that name. It returns `None` otherwise, and the caller asks
/// clap for help instead.
///
/// The words that name something are the words before any `--` that do not
/// start with `-`.
//
// RUST-10: the page ignores flags, so `shelf help catalogue --json` prints
// the topic as text. A topic in JSON replaces this when a caller needs it.
fn help_page(command: &Command, guide: &Guide, rest: &[OsString]) -> Option<String> {
    let mut words = rest
        .iter()
        .take_while(|word| *word != "--")
        .filter(|word| !word.as_encoded_bytes().starts_with(b"-"));
    let root = guide.root_in(command).iter().fold(command, |node, name| {
        node.find_subcommand(name).unwrap_or(node)
    });
    match (words.next(), words.next()) {
        (None, _) => Some(topic::list(&guide.invocation(command), guide.get_topics())),
        (Some(word), None) if root.find_subcommand(word).is_none() => {
            topic::text(command.get_name(), guide, word.to_str()?)
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
