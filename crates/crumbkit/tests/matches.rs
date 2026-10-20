//! A tool outside crumbkit that keeps clap's matches after parsing, so it
//! can read where each value came from without parsing the command line a
//! second time.

use clap::parser::ValueSource;
use clap::{ArgMatches, CommandFactory as _, FromArgMatches, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Sink, ViewFlags};

/// A tool with a `list` command.
#[derive(Debug, Parser)]
#[command(name = "tool")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Debug, PartialEq, Eq, Subcommand)]
enum Command {
    /// Lists the books.
    List {
        /// How many books to list.
        #[arg(long, default_value_t = 10)]
        limit: u32,
        /// Lists the borrowed books too.
        #[arg(long)]
        all: bool,
    },
}

/// The tool's parsed type with a copy of the matches that it took its
/// values from.
struct WithMatches {
    cli: Cli,
    matches: ArgMatches,
}

impl FromArgMatches for WithMatches {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        Self::from_arg_matches_mut(&mut matches.clone())
    }

    fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<Self, clap::Error> {
        // `Cli` takes each value out of `matches`, so the copy is made first.
        let kept = matches.clone();
        Cli::from_arg_matches_mut(matches).map(|cli| Self { cli, matches: kept })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        self.update_from_arg_matches_mut(&mut matches.clone())
    }

    fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), clap::Error> {
        let kept = matches.clone();
        self.cli.update_from_arg_matches_mut(matches)?;
        self.matches = kept;
        Ok(())
    }
}

#[test]
fn a_tool_keeps_the_matches_by_cloning_them_before_its_type_takes_the_values() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let parsed = crumbkit::try_parse_to::<WithMatches, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Cli::command(),
        &Guide::new(),
        ["tool", "list", "--all"],
    );

    let Ok((WithMatches { cli, matches }, _output)) = parsed else {
        panic!("the line did not parse: {out:?} {err:?}");
    };
    let list = matches.subcommand_matches("list");
    assert_eq!(
        list.and_then(|list| list.value_source("all")),
        Some(ValueSource::CommandLine)
    );
    assert_eq!(
        list.and_then(|list| list.value_source("limit")),
        Some(ValueSource::DefaultValue)
    );
    assert_eq!(
        cli.command,
        Command::List {
            limit: 10,
            all: true
        }
    );
}
