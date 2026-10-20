//! `parse`, which parses a tool's command line with crumbkit's help lines,
//! and prints help, the version or a refusal in crumbkit's formats.

use std::ffi::OsString;
use std::io::{BufRead, IsTerminal as _, Write};

use clap::error::ErrorKind;
use clap::{Command, FromArgMatches};

pub use self::refused::{Refused, Unparsed};
pub use self::stop::Stop;

use self::attempt::{Attempt, attempt};
use self::help_word::{ask_for_help, has_topics, topic_page};
use self::lines::add_help_lines;
use self::message::refusal_message;
use self::page::{print_page, print_text};
use self::pick::{Picking, Step};
use self::walk::{help_word, own_names, program_stem, walk};
use crate::help::Guide;
use crate::refusal::Refusal;
use crate::{Output, Sink, Spelling};

mod attempt;
mod declared;
mod help_word;
mod lines;
mod message;
mod page;
mod pick;
mod refused;
mod stop;
pub(crate) mod suggestion;
#[cfg(test)]
mod tests;
pub(crate) mod walk;

/// Parses the process's command line against `command` with the tool's
/// `guide`, and returns the tool's parsed type and an [`Output`], which
/// holds the view flags, the command that it parsed against, the words
/// that the caller typed and what `guide` decides about printing.
///
/// A tool's `main` calls it in place of clap's `Cli::parse`, and passes the
/// returned `Output` to [`finish`](crate::finish()) with the handler's
/// result. `parse` reads the view flags that the tool's type flattens as
/// [`ViewFlags`](crate::ViewFlags) by their ids, before the tool's type
/// takes them, and reads [`DryRun`](crate::DryRun)'s `--dry-run` from the
/// deepest command in the same way, which
/// [`Output::is_dry_run`](crate::Output::is_dry_run) reports. A tool
/// without goal headings or topics passes `&Guide::new()`:
///
/// ```no_run
/// use std::process::ExitCode;
///
/// use clap::{CommandFactory, Parser};
/// use crumbkit::help::Guide;
/// use crumbkit::ViewFlags;
///
/// /// List the books in a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {
///     #[command(flatten)]
///     view: ViewFlags,
/// }
///
/// fn main() -> ExitCode {
///     let parsed = crumbkit::parse::<Cli>(Cli::command(), &Guide::new());
///     let (_cli, _output) = match parsed {
///         Ok(parsed) => parsed,
///         Err(stop) => return stop.code(),
///     };
///     // Run the command, then return `crumbkit::finish(output, ...)`.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// It behaves as [`parse_to`] does with a sink over the process's standard
/// output and standard error that asks for styles when standard output is a
/// terminal and the guide detects terminals ([`Sink::styled`]). Help then
/// prints through clap's `Error::print`, which keeps clap's colours unless
/// `NO_COLOR` or the command's `ColorChoice` turns them off. clap prints those
/// colours only when the tool enables clap's `color` feature.
///
/// `parse` is [`try_parse`] followed by printing the refusal. A tool that
/// reads a refused command line before anything prints, such as a tool
/// that resolves its own aliases or prints its own failure document, calls
/// `try_parse`.
///
/// # Errors
///
/// Returns a [`Stop`] after crumbkit has printed help, the version or a
/// refusal. It holds the exit code for `main` to return, and whether the
/// caller asked for JSON.
pub fn parse<C: FromArgMatches>(command: Command, guide: &Guide) -> Result<(C, Output), Stop> {
    parse_from(command, guide, std::env::args_os())
}

/// Parses the process's command line as [`parse`] does, and returns a
/// refused command line unprinted, as a [`Refused`] inside
/// [`Unparsed::Refused`].
///
/// A tool calls it when it needs the refusal before anything prints:
///
/// ```no_run
/// use std::process::ExitCode;
///
/// use clap::{CommandFactory, Parser};
/// use crumbkit::help::Guide;
/// use crumbkit::{Unparsed, ViewFlags};
///
/// /// List the books in a small catalogue.
/// #[derive(Parser)]
/// #[command(name = "shelf")]
/// struct Cli {
///     #[command(flatten)]
///     view: ViewFlags,
/// }
///
/// fn main() -> ExitCode {
///     let parsed = crumbkit::try_parse::<Cli>(Cli::command(), &Guide::new());
///     let (_cli, _output) = match parsed {
///         Ok(parsed) => parsed,
///         Err(Unparsed::Refused(refused)) => {
///             // The tool reads `refused.typed()` and `refused.culprit()`
///             // here, and prints the refusal when it has no other answer.
///             return refused.print().code();
///         }
///         Err(unparsed) => return unparsed.print().code(),
///     };
///     // Run the command, then return `crumbkit::finish(output, ...)`.
///     ExitCode::SUCCESS
/// }
/// ```
///
/// # Errors
///
/// Returns [`Unparsed::Stopped`] after crumbkit has printed help, the
/// version or a topic, and [`Unparsed::Refused`] for a refused command line,
/// which nothing has printed.
pub fn try_parse<C: FromArgMatches>(
    command: Command,
    guide: &Guide,
) -> Result<(C, Output), Unparsed> {
    try_parse_from(command, guide, std::env::args_os())
}

/// Parses `args` against `command` as [`parse`] parses the process's command
/// line. The first argument is the program name, as in clap's `parse_from`.
///
/// # Errors
///
/// Returns a [`Stop`] after crumbkit has printed help, the version or a
/// refusal. It holds the exit code for `main` to return, and whether the
/// caller asked for JSON.
pub fn parse_from<C, I, T>(command: Command, guide: &Guide, args: I) -> Result<(C, Output), Stop>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    try_parse_from(command, guide, args).map_err(Unparsed::print)
}

/// Parses `args` against `command` as [`try_parse`] parses the process's
/// command line. The first argument is the program name.
///
/// # Errors
///
/// Returns [`Unparsed::Stopped`] after crumbkit has printed help, the
/// version or a topic, and [`Unparsed::Refused`] for a refused command line,
/// which nothing has printed.
pub fn try_parse_from<C, I, T>(
    command: Command,
    guide: &Guide,
    args: I,
) -> Result<(C, Output), Unparsed>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let sink = Sink::standard();
    let styled = guide.detects(|| sink.out.is_terminal());
    run(&mut sink.styled(styled), command, guide, args, true)
}

/// Parses `args` against `command` and prints help, the version or a
/// refusal to `sink`, and returns what [`parse_from`] would return. It is
/// [`try_parse_to`] followed by [`Unparsed::print_to`] on `sink`.
///
/// Tests and golden files call it with a sink over in-memory writers, as
/// they call [`finish_to`](crate::finish_to). Help and the version print as
/// plain text, unless the sink asks for styles with [`Sink::styled`]. A
/// styled sink receives help with the ANSI escape codes that clap's
/// `Error::print` writes at a terminal, when crumbkit's `color` feature is
/// on. A topic prints as plain text to every sink.
///
/// Before parsing, crumbkit walks the typed words down the tree to the
/// deepest command they name, and changes the help of each command on the
/// way, the root included. The walk reads each option's values as clap's
/// parser does, from the option's `Arg`, so an option's value that equals a
/// subcommand name, as in `shelf --fields books`, names no command.
///
/// When `guide` declares [`Guide::dashed_forms`], a command that the tool
/// named in another style, such as `reading_lists` or `ReadingLists`, also
/// answers to its [`dashed_form`](crate::dashed_form), such as
/// `reading-lists`. Before anything else reads the words, crumbkit replaces each
/// word that reaches a command by its dashed form with the command's own
/// name, so help, refusals, the usage log and the words in the returned
/// [`Output`] name the command as the tool does. An option's value that
/// equals a dashed form stays as typed.
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
///   column sections. A headed document's line says `explains each JSON
///   field`, and its section lists the JSON fields that `--fields` selects.
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
/// - `shelf help` prints what `shelf --help` prints, whose `Topics:` line
///   names each topic.
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
///   `--help`. crumbkit puts `--help` directly after the last word that
///   follows `help`, comes before any `--` and does not start with `-`, or
///   in the place of `help` when no word qualifies. clap then meets
///   `--help` before the flags that follow the command's words. `shelf help
///   books show` and `shelf books help show` print what `shelf books show
///   --help` prints, `shelf books help` prints what `shelf books --help`
///   prints, `shelf help --json` prints what `shelf --help --json` prints,
///   and clap refuses a word that names no command, as in `shelf help
///   comets` or `shelf books help shwo`.
///
/// A command without subcommands reads `help` as an ordinary word, such as
/// the value of a positional argument. A command with its own subcommand
/// named or aliased `help` lets clap parse that subcommand.
///
/// Help, the version and a topic print to the sink's output writer, and the
/// exit code is 0, also when the reader closed the pipe early.
///
/// [`try_parse_to`] returns a refused command line unprinted, and
/// [`Refused::print_to`] prints it as the paragraphs below describe.
///
/// A command line that clap refuses prints as a failure with the code
/// `<tool>::cli::usage` in the guide's spelling, and the exit code is 2.
/// The message is clap's error on one line, and the recovery runs the `-h`
/// of the deepest command. When the tool's refusal function returns a call
/// that renders and whose line parses, the first such call is the recovery
/// and the `-h` call is the first alternative, as shelf's golden file
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
/// one `<tool>/failure/v1` document, as a failure does with `finish`. The
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
    args: I,
) -> Result<(C, Output), Stop>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    O: Write,
    E: Write,
{
    try_parse_to(sink, command, guide, args).map_err(|unparsed| unparsed.print_to(sink))
}

/// Parses `args` against `command` as [`parse_to`] does, and prints help,
/// the version or a topic to `sink`. It returns a refused command line
/// unprinted, as a [`Refused`] inside [`Unparsed::Refused`], which
/// [`Refused::print_to`] prints as `parse_to` prints it.
///
/// A refused `help` command line keeps both of its forms. `shelf help
/// comets` is refused as `shelf comets --help`, so [`Refused::typed`]
/// returns `help` and `comets`, [`Refused::parsed`] returns `comets` and
/// `--help`, and [`Refused::culprit`] returns `comets`.
///
/// # Errors
///
/// Returns [`Unparsed::Stopped`] after crumbkit has printed help, the
/// version or a topic, and [`Unparsed::Refused`] for a refused command line,
/// which nothing has printed.
pub fn try_parse_to<C, I, T, O, E>(
    sink: &mut Sink<O, E, impl BufRead>,
    command: Command,
    guide: &Guide,
    args: I,
) -> Result<(C, Output), Unparsed>
where
    C: FromArgMatches,
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
    O: Write,
    E: Write,
{
    run(sink, command, guide, args, false)
}

/// Parses `args` against `command` as [`parse_to`] describes, without
/// printing a refusal. With the `tracing` feature, parsing runs inside the
/// `crumbkit.parse` span, which records the exit code when parsing ends the
/// run (ADR-0030). A refusal keeps the span, and records its exit code there
/// when it prints.
fn run<C, I, T, O, E, R>(
    sink: &mut Sink<O, E, R>,
    command: Command,
    guide: &Guide,
    args: I,
    standard: bool,
) -> Result<(C, Output), Unparsed>
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
    // Codes print in the guide's spelling, and crumbkit's own codes print
    // with the tool's name in place of `crumbkit` (ADR-0079).
    let spelling = guide.get_spelling().for_tool(guide.tool_name(&command));
    let parsed = read(sink, command, guide, spelling, args, standard);
    #[cfg(feature = "tracing")]
    if let Err(Unparsed::Stopped(stop)) = &parsed {
        crate::trace::spans::exited(&span, stop.number());
    }
    parsed
}

/// Parses `args` against `command` as [`parse_to`] describes, and returns a
/// refusal unprinted. `standard` says that `sink` is over the process's
/// standard streams, as in [`parse_from`]: help for a styled sink then
/// prints through clap's `Error::print`, and the guide's picker reads the
/// process's standard input and writes to its standard error. Otherwise
/// help goes to the sink's output writer, and the picker uses the sink's
/// input and error writer.
fn read<C, I, T, O, E, R>(
    sink: &mut Sink<O, E, R>,
    mut command: Command,
    guide: &Guide,
    spelling: Spelling,
    args: I,
    standard: bool,
) -> Result<(C, Output), Unparsed>
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
    let replaced = own_names(&command, guide.get_dashed_forms(), &mut args);
    // A refusal reports the words as the caller typed them. Only a run
    // that changes a word keeps a copy, so a run that types the tool's own
    // names and no `help` allocates no copy.
    let mut typed = (!replaced.is_empty()).then(|| typed_words(&args, replaced));
    crate::events::typed(&command, &args, spelling);
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
    let has_topics = has_topics(&command, root, guide);
    if let Some(help) = help_word(&command, root, has_topics, &args) {
        if help.at_root
            && let Some(page) = topic_page(&command, guide, &args[help.position + 1..])
        {
            // RUST-10: a topic prints as plain text, also on a terminal and
            // to a styled sink, because crumbkit builds it without styles
            // and only the pages that clap renders carry them. A topic with
            // the root's heading and literal styles, written as `print_page`
            // writes help to a styled sink, replaces this when a tool needs
            // colour there.
            let code = print_text(sink, &page);
            if crate::events::observed() {
                let root: Vec<String> = root.iter().map(|&name| name.to_owned()).collect();
                crate::events::helped(&command, &root, true, code);
            }
            // Only this path ends before the walk below, so it walks the
            // words for `--json` here.
            let json = walk(&command, &args).json;
            return Err(Unparsed::Stopped(Stop::new(code, json, command, guide)));
        }
        typed.get_or_insert_with(|| args.get(1..).unwrap_or_default().to_vec());
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
        let mut picking = Picking::new(picker, standard);
        let mut picked = 0;
        while let Err(error) = &parsed
            && pick::wanted(error)
        {
            typed.get_or_insert_with(|| words.clone());
            match picking.pick(sink, &mut command, guide, &mut words, json, spelling) {
                Step::Picked => picked += 1,
                Step::Refuse => break,
                Step::Stop(code) => {
                    return Err(Unparsed::Stopped(Stop::new(code, json, command, guide)));
                }
            }
            let args = pick::args(&command, &words);
            parsed = attempt::<C>(&mut command, args, guide);
        }
        if picked > 0 && parsed.is_ok() {
            pick::echo(sink, &command, &words);
            crate::events::picked(picked);
        }
    }
    let error = match parsed {
        Ok(Attempt {
            parsed,
            view: Ok(view),
            no_input,
            dry_run,
        }) => {
            return Ok((
                parsed,
                Output::parsed(view, command, words, dry_run, no_input, guide, &path),
            ));
        }
        Ok(Attempt {
            view: Err(refusal), ..
        }) => {
            let output = stop::stopped(json, command, guide);
            let typed = typed.unwrap_or_else(|| words.clone());
            let refused = Refused::new(*refusal, None, path, output, typed, words);
            return Err(Unparsed::Refused(refused));
        }
        Err(error) => error,
    };
    if matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        let code = print_page(sink, &error, standard);
        if error.kind() == ErrorKind::DisplayHelp {
            crate::events::helped(&command, &path, long_help, code);
        } else {
            crate::events::exited(code);
        }
        return Err(Unparsed::Stopped(Stop::new(code, json, command, guide)));
    }
    let message = refusal_message(&command, &path, &error);
    let declared = declared::alternatives::<C>(&mut command, guide, &path, &error, &words);
    let refusal = Refusal::command_line(message, path.clone(), &error, &words, declared);
    let output = stop::stopped(json, command, guide);
    let typed = typed.unwrap_or_else(|| words.clone());
    let refused = Refused::new(refusal, Some(error), path, output, typed, words);
    Err(Unparsed::Refused(refused))
}

/// Returns the words after the program name in `args` as the caller typed
/// them, with each word of `replaced`, the positions in `args` and the
/// typed text that [`own_names`] replaced, put back.
fn typed_words(args: &[OsString], replaced: Vec<(usize, OsString)>) -> Vec<OsString> {
    let mut typed = args.to_vec();
    for (position, word) in replaced {
        if let Some(slot) = typed.get_mut(position) {
            *slot = word;
        }
    }
    typed.remove(0);
    typed
}
