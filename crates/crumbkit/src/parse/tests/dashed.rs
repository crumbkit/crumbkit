//! Tests for a command named in another style, which also answers to its
//! dashed form.

use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Arg, ArgMatches, Command, FromArgMatches};

use super::parse_tree;
use crate::help::Guide;
use crate::parse::parse_to;
use crate::{Sink, Spelling};

/// A tool with a global `--fields` option and a `reading_lists` resource
/// that holds `list`.
fn tool() -> Command {
    Command::new("tool")
        .arg(Arg::new("fields").long("fields").global(true))
        .subcommand(Command::new("reading_lists").subcommand(Command::new("list")))
}

/// The names of the subcommands that clap matched, from the root down.
struct Matched(Vec<String>);

impl FromArgMatches for Matched {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut names = Vec::new();
        let mut matches = matches;
        while let Some((name, below)) = matches.subcommand() {
            names.push(name.to_owned());
            matches = below;
        }
        Ok(Self(names))
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

#[test]
fn a_dashed_form_reaches_the_command_and_the_parsed_words_hold_the_tool_s_own_name() {
    let words = ["--fields", "reading-lists", "reading-lists", "list"];
    let result = parse_to::<Matched, _, _, _, _>(
        &mut Sink::new(Vec::new(), Vec::new()),
        tool(),
        &Guide::new(),
        Spelling::COLONS,
        ["tool"].iter().chain(&words),
    );

    let Ok((matches, parsed)) = result else {
        panic!("the command line did not parse");
    };
    assert_eq!(matches.0, ["reading_lists", "list"]);
    let own = ["--fields", "reading-lists", "reading_lists", "list"].map(OsString::from);
    assert_eq!(parsed.words, Some(own.to_vec()));
}

#[test]
fn help_reached_by_a_dashed_form_names_the_command_by_the_tool_s_own_name_only() {
    for args in [
        ["tool", "reading-lists", "-h"].as_slice(),
        &["tool", "help", "reading-lists"],
    ] {
        let parsed = parse_tree(tool(), args);

        assert_eq!(parsed.code, Some(ExitCode::SUCCESS), "{args:?}");
        assert!(
            parsed.out.contains("Usage: tool reading_lists"),
            "{}",
            parsed.out
        );
        assert!(!parsed.out.contains("reading-lists"), "{}", parsed.out);
    }
}
