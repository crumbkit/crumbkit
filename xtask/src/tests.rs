//! Tests for the command line and for the repository itself.

use clap::{CommandFactory, Parser};

use super::{Cli, Scope, read_files};
use crate::check::{self, Selection};

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("xtask").chain(args.iter().copied()))
}

#[test]
fn the_command_line_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn check_alone_selects_every_check() {
    assert_eq!(parse(&["check"]).unwrap(), Cli::Check { scope: None });
}

#[test]
fn check_docs_selects_only_the_docs_check() {
    assert_eq!(
        parse(&["check", "docs"]).unwrap(),
        Cli::Check {
            scope: Some(Scope::Docs)
        }
    );
}

#[test]
fn ci_time_selects_the_ci_time_check() {
    assert_eq!(parse(&["ci-time"]).unwrap(), Cli::CiTime);
}

#[test]
fn any_other_command_line_is_refused() {
    assert!(parse(&[]).is_err());
    assert!(parse(&["docs"]).is_err());
    assert!(parse(&["check", "rust"]).is_err());
    assert!(parse(&["check", "docs", "extra"]).is_err());
    assert!(parse(&["ci-time", "extra"]).is_err());
}

#[test]
fn the_repository_passes_every_check() {
    let files = read_files().unwrap();

    assert_eq!(check::run(Selection::All, &files), []);
}
