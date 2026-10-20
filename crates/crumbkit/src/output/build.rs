//! `ViewFlags`, the view flags that a tool flattens into its clap parser or
//! builds itself, and `Output::new`, which builds the output of a run from
//! them.

use std::ffi::OsString;
use std::io::Write;

use clap::{ArgMatches, Args, Command};

use super::failure::Printing;
use super::{DryRun, Output, budget};
use crate::Sink;
use crate::help::Guide;
use crate::help::rename::Renames;
use crate::parse::{Printed, Stop};
use crate::view::View;

/// The flags that choose how a command prints its result: `--json`,
/// `--fields`, `--long` and `--full`.
///
/// A tool adds them to its own clap parser with `#[command(flatten)]`. Every
/// flag is global, so it also works after a subcommand: `shelf books list
/// --json`. `ViewFlags` adds no description to the command that flattens
/// it, and nor does any other argument type of crumbkit, so a command
/// without a doc comment of its own has none in its help.
///
/// - `--json` prints a JSON document, on one line unless the guide sets
///   [`JsonLayout::Pretty`](crate::JsonLayout::Pretty).
/// - `--long` adds the row type's [`Row::LONG_COLUMNS`](crate::Row::LONG_COLUMNS).
/// - `--full` prints every field of every row in declaration order, and needs `--json`.
/// - `--fields a,b` prints exactly the named fields in that order, and
///   conflicts with `--long` and `--full`.
///
/// [`parse`](crate::parse()) reads the four flags from clap's matches by
/// their ids, `json`, `fields`, `long` and `full`, before the tool's type
/// takes the values, and puts them in the [`Output`] that it returns. The
/// tool's own copy of `ViewFlags` is then unused. A tool that defines the
/// flags in its own parser gives its arguments those ids and marks them
/// global, and `parse` reads them the same way. A tool without the flags
/// parses with every view flag off.
///
/// A tool that parses with clap alone passes the `ViewFlags` that clap
/// parsed to [`Output::new`], which refuses `--full` without `--json` and
/// `--fields` with `--long` or `--full` (ADR-0093). A tool that parses the
/// flags its own way builds them with [`ViewFlags::new`] and the builders:
///
/// ```
/// use crumbkit::ViewFlags;
///
/// let view = ViewFlags::new()
///     .json(true)
///     .fields(vec!["id".to_owned(), "title".to_owned()]);
/// ```
#[derive(Args, Clone, Debug, Default)]
// The doc comment is rustdoc, and clap would copy it into the help of a
// command without a description of its own.
#[command(about = None, long_about = None)]
pub struct ViewFlags {
    /// Print the result as a JSON document
    #[arg(long, global = true)]
    pub(crate) json: bool,

    /// Print only these comma-separated fields, in this order
    #[arg(long, global = true, value_name = "FIELD", value_delimiter = ',')]
    pub(crate) fields: Vec<String>,

    /// Add the long columns after the compact ones
    #[arg(long, global = true)]
    pub(crate) long: bool,

    /// With --json, print every field of every row
    #[arg(long, global = true)]
    pub(crate) full: bool,
}

impl ViewFlags {
    /// Returns the view flags with every flag off, the same value as
    /// `ViewFlags::default()`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            json: false,
            fields: Vec::new(),
            long: false,
            full: false,
        }
    }

    /// Returns the flags with `--json` set to `on`.
    #[must_use]
    pub const fn json(mut self, on: bool) -> Self {
        self.json = on;
        self
    }

    /// Returns the flags with `--long` set to `on`.
    #[must_use]
    pub const fn long(mut self, on: bool) -> Self {
        self.long = on;
        self
    }

    /// Returns the flags with `--full` set to `on`.
    #[must_use]
    pub const fn full(mut self, on: bool) -> Self {
        self.full = on;
        self
    }

    /// Returns the flags with `--fields` naming `names`, in that order. An
    /// empty list means that the caller passed no `--fields`. It is not a
    /// `const fn`, because a `const fn` cannot drop the list it replaces.
    #[must_use]
    pub fn fields(mut self, names: Vec<String>) -> Self {
        self.fields = names;
        self
    }

    /// Returns the view flags in `matches`, the matches of `node`, the
    /// deepest command that the caller typed.
    ///
    /// clap copies the value of a global argument to every command on the
    /// path, so `matches` holds a flag that the caller typed before the
    /// command path too. A flag counts only when `node`'s argument with the
    /// flag's id is global and holds the type that `ViewFlags` gives it, so
    /// a tool's own argument with that id is left alone.
    pub(crate) fn read(node: &Command, matches: &ArgMatches) -> Self {
        let global = |id: &str| {
            node.get_arguments()
                .any(|arg| arg.get_id() == id && arg.is_global_set())
        };
        // The `try_` getters return an error for an id that the matches do
        // not know or a value of another type, where the plain getters
        // panic.
        let flag =
            |id: &str| global(id) && matches.try_get_one::<bool>(id).ok().flatten() == Some(&true);
        let fields = if global("fields") {
            matches
                .try_get_many::<String>("fields")
                .ok()
                .flatten()
                .map(|names| names.cloned().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        Self {
            json: flag("json"),
            fields,
            long: flag("long"),
            full: flag("full"),
        }
    }
}

impl Output {
    /// Returns the output of a run that parsed `args`, the command line
    /// with the program name first, against `command` with clap alone,
    /// with the view flags `view`, the dry-run flag `dry_run` and the
    /// tool's `guide`, or the [`Stop`] of a run whose view flags cannot
    /// work together.
    ///
    /// [`parse`](crate::parse()) returns the `Output` of a tool that parses
    /// with crumbkit. A tool that parses with clap alone, or that parses
    /// the view flags its own way, builds it here:
    ///
    /// ```
    /// use clap::{CommandFactory, Parser};
    /// use crumbkit::help::Guide;
    /// use crumbkit::{DryRun, Output, ViewFlags};
    ///
    /// /// List the books in a small catalogue.
    /// #[derive(Parser)]
    /// struct Cli {
    ///     #[command(flatten)]
    ///     view: ViewFlags,
    /// }
    ///
    /// let args = ["shelf", "--json"];
    /// let cli = Cli::try_parse_from(args).unwrap();
    /// let output = Output::new(cli.view, DryRun::default(), Cli::command(), &Guide::new(), args);
    /// assert!(output.unwrap().is_json());
    /// ```
    ///
    /// `dry_run` is the [`DryRun`] that the command parsed, which
    /// [`Output::is_dry_run`] reports and `finish` marks the result with.
    /// A tool without the flag passes `DryRun::default()`, and the run is
    /// no dry run.
    ///
    /// The `Output` keeps `command`, which `finish` renders a listing's next
    /// step and a failure's [`Recovery::Run`](crate::Recovery::Run) call
    /// against, and the words of `args` after the program name, which
    /// [`Output::confirm`] repeats with `--yes`. It reads the terminal
    /// detection, output budget, JSON layout, exit codes, code renames and
    /// [spelling](Guide::spelling) of `guide`, so `finish` and `stream` print
    /// every code in the guide's spelling with the tool's name in crumbkit's
    /// own codes (ADR-0086). The renames of the leaf
    /// that the words name, as [`command_path`](crate::command_path) finds
    /// it, come before the guide's (ADR-0060), so a tool that parses with
    /// clap alone prints a leaf's own codes as one that parses with
    /// crumbkit does (ADR-0076).
    ///
    /// The `Output` reads no `--no-input` from the words, so
    /// [`Output::confirm`] fails without asking only when no person can
    /// answer at a terminal.
    ///
    /// A call into a branch below the root resolves the root's global
    /// arguments, such as `--json`, only when the root was built by
    /// parsing. A tree that never went through parsing, such as a fresh
    /// `Cli::command()` beside clap's `Cli::parse`, leaves out a `Next:`
    /// line or a recovery whose call names one.
    ///
    /// `finish` and `stream` check the view flags against the result as
    /// `parse` does: they refuse a view flag that the result cannot use and
    /// an unknown `--fields` name, after the handler has run.
    ///
    /// # Errors
    ///
    /// It returns the `Stop` of a refused command line, as [`parse`](crate::parse())
    /// does, when `--full` comes without `--json` or `--fields` comes with
    /// `--long` or `--full` (ADR-0093). The refusal prints on standard
    /// error, as text or with `--json` as one JSON document, before the
    /// tool's handler runs, and `main` returns the stop's code. A tool that
    /// prints the refusal elsewhere calls [`Output::new_to`].
    pub fn new<I, T>(
        view: ViewFlags,
        dry_run: DryRun,
        command: Command,
        guide: &Guide,
        args: I,
    ) -> Result<Self, Stop>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        Self::new_to(&mut Sink::standard(), view, dry_run, command, guide, args)
    }

    /// Returns the output of a run as [`Output::new`] does, and prints the
    /// refusal of view flags that cannot work together to `sink` in place
    /// of standard error.
    ///
    /// # Errors
    ///
    /// It returns the `Stop` that [`Output::new`] returns, after it printed
    /// the refusal to `sink`.
    pub fn new_to<I, T, O, E, R>(
        sink: &mut Sink<O, E, R>,
        view: ViewFlags,
        dry_run: DryRun,
        command: Command,
        guide: &Guide,
        args: I,
    ) -> Result<Self, Stop>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
        O: Write,
        E: Write,
    {
        let json = view.json;
        let words: Vec<OsString> = args.into_iter().skip(1).map(Into::into).collect();
        match View::try_from(view) {
            Ok(view) => {
                let path = crate::command_path(&command, guide, &words);
                let dry_run = dry_run.is_on();
                Ok(Self::parsed(
                    view, command, words, dry_run, false, guide, &path,
                ))
            }
            Err(refusal) => {
                let mut command = command;
                let spelling = guide.get_spelling().for_tool(guide.tool_name(&command));
                let printing = Printing::of(guide);
                crate::events::exited(printing.exit(&crate::codes::CLI_USAGE));
                let code = refusal.print(sink, json, spelling, printing, &mut command);
                let path = crate::command_path(&command, guide, &words);
                Err(Stop::new(
                    Printed::Refusal,
                    code,
                    json,
                    command,
                    guide,
                    path,
                    words,
                ))
            }
        }
    }

    /// Returns the output of a run that parsed `words`, the words after the
    /// program name, against `command`, with the view flags `view`, whether
    /// the caller passed `--dry-run` and `--no-input`, and the terminal
    /// detection, output budget, JSON layout, exit codes and renames of
    /// `guide`. The renames of the guide's leaf at `path`, the real names of
    /// the parsed command's subcommands from the root, come before the
    /// guide's (ADR-0060).
    pub(crate) fn parsed(
        view: View,
        command: Command,
        words: Vec<OsString>,
        dry_run: bool,
        no_input: bool,
        guide: &Guide,
        path: &[String],
    ) -> Self {
        let leaf = guide
            .get_leaves()
            .iter()
            .find(|leaf| leaf.path().iter().eq(path.iter()));
        Self {
            words,
            dry_run,
            no_input,
            renames: Renames::with_leaf(guide, leaf),
            ..Self::stopped(view, command, guide)
        }
    }

    /// Replaces the view with `view`. A refusal that the tool printed in a
    /// form it chose uses it, so the `Stop` reports the printed form.
    pub(crate) fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// Returns the output of a run with the view `view` that parsed against
    /// `command` with `guide`, without words. `parse` uses it after it
    /// printed help, the version or a refusal.
    pub(crate) fn stopped(view: View, command: Command, guide: &Guide) -> Self {
        let tool = guide.tool_name(&command);
        let budget = match guide.get_output_budget() {
            Some((bytes, _)) => bytes,
            None => budget::DEFAULT,
        };
        Self {
            view,
            command,
            words: Vec::new(),
            dry_run: false,
            no_input: false,
            no_terminal: !guide.terminal_detection(),
            terminal_width: None,
            json_layout: guide.get_json_layout(),
            budget: budget::from_environment(budget, tool),
            spelling: guide.get_spelling().for_tool(tool),
            exits: guide.get_exits(),
            renames: Renames::of(guide),
        }
    }

    /// Returns the output of a run of `command` without view flags, words
    /// or a guide, for a test that needs any output.
    #[cfg(test)]
    pub(crate) fn of(command: Command) -> Self {
        Self::stopped(
            View::Text(crate::view::Table::Compact),
            command,
            &Guide::new(),
        )
    }
}
