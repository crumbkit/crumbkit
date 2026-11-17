//! Tests for `Lazy`: when a group builds its subcommands, and what it
//! parses to.

use std::sync::atomic::{AtomicUsize, Ordering};

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};

use super::Lazy;
use crate::{Sink, Tool, reach};

/// How many times a `Counted` group built its subcommands.
static BUILDS: AtomicUsize = AtomicUsize::new(0);

/// A group whose `augment_subcommands` counts its calls.
#[derive(Debug, PartialEq, Eq)]
enum Counted {
    Install,
}

impl FromArgMatches for Counted {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        match matches.subcommand_name() {
            Some("install") => Ok(Self::Install),
            _ => Err(clap::Error::new(clap::error::ErrorKind::MissingSubcommand)),
        }
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Subcommand for Counted {
    fn augment_subcommands(command: clap::Command) -> clap::Command {
        BUILDS.fetch_add(1, Ordering::SeqCst);
        command.subcommand(clap::Command::new("install"))
    }

    fn augment_subcommands_for_update(command: clap::Command) -> clap::Command {
        Self::augment_subcommands(command)
    }

    fn has_subcommand(name: &str) -> bool {
        name == "install"
    }
}

#[derive(Debug, Parser)]
#[command(name = "tool")]
struct Counting {
    #[command(subcommand)]
    command: Top,
}

#[derive(Debug, Subcommand)]
enum Top {
    Books,
    Skills {
        #[command(subcommand)]
        command: Lazy<Counted>,
    },
}

/// What a group parses to, built eagerly and lazily.
#[derive(Debug, Subcommand, PartialEq, Eq)]
enum Verbs {
    Install(Install),
    Check,
}

#[derive(Debug, Args, PartialEq, Eq)]
struct Install {
    #[arg(long)]
    dir: Option<String>,
}

#[derive(Debug, Parser, PartialEq, Eq)]
#[command(name = "tool")]
struct Eager {
    #[command(subcommand)]
    command: EagerTop,
}

#[derive(Debug, Subcommand, PartialEq, Eq)]
enum EagerTop {
    Skills {
        #[command(subcommand)]
        command: Verbs,
    },
}

#[derive(Debug, Parser)]
#[command(name = "tool")]
struct Deferred {
    #[command(subcommand)]
    command: DeferredTop,
}

#[derive(Debug, Subcommand)]
enum DeferredTop {
    Skills {
        #[command(subcommand)]
        command: Lazy<Verbs>,
    },
}

fn parse_counting(args: &[&str]) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let _ = crate::parse::try_parse_to::<Counting>(
        &mut Sink::new(&mut out, &mut err),
        Counting::command(),
        &Tool::new(),
        args,
    );
}

#[test]
fn a_lazy_group_adds_no_subcommands_until_its_word_is_typed() {
    parse_counting(&["tool", "-h"]);
    parse_counting(&["tool", "books"]);
    assert_eq!(BUILDS.load(Ordering::SeqCst), 0);

    parse_counting(&["tool", "skills", "install"]);
    assert_eq!(BUILDS.load(Ordering::SeqCst), 1);
}

#[test]
fn a_lazy_group_s_leaf_parses_to_the_same_value_as_an_eager_one() {
    for words in [
        &["tool", "skills", "install", "--dir", "x"][..],
        &["tool", "skills", "check"],
    ] {
        let eager = Eager::try_parse_from(words).unwrap();
        let mut command = Deferred::command();
        reach(&mut command, &words[1..]);
        let matches = command.try_get_matches_from(words).unwrap();
        let lazy = Deferred::from_arg_matches(&matches).unwrap();
        let DeferredTop::Skills { command: group } = lazy.command;
        let EagerTop::Skills { command: expected } = eager.command;
        assert_eq!(group.into_inner(), expected, "{words:?}");
    }
}

#[test]
fn a_lazy_group_shows_the_same_help_as_an_eager_one() {
    let mut command = Deferred::command();
    reach(&mut command, &["skills"]);
    let skills = command.find_subcommand_mut("skills").unwrap();
    let lazy = skills.render_help().to_string();
    let mut eager = Eager::command().disable_help_subcommand(true);
    let _ = eager.render_usage();
    let expected = eager
        .find_subcommand_mut("skills")
        .unwrap()
        .render_help()
        .to_string();
    assert_eq!(lazy, expected);
}

#[test]
fn a_lazy_group_keeps_a_value_name_that_the_tool_chose() {
    let command = Lazy::<Verbs>::augment_subcommands(clap::Command::new("skills"));
    assert_eq!(command.get_subcommand_value_name(), Some("COMMAND"));
    let named = Lazy::<Verbs>::augment_subcommands(
        clap::Command::new("skills").subcommand_value_name("VERB"),
    );
    assert_eq!(named.get_subcommand_value_name(), Some("VERB"));
}
