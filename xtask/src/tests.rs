//! Tests for the command line and for the repository itself.
//!
//! The tests build each `ADR-` citation with `format!`, so this file contains
//! no citation of its own for the repository check to find.

use clap::{CommandFactory, Parser};

use super::{Cli, Scope, read_files};
use crate::check::{self, File, Selection, Violation};

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

/// Returns the repository's file at `path` from `files`.
fn file_at<'a>(files: &'a mut [File], path: &str) -> &'a mut File {
    files
        .iter_mut()
        .find(|file| file.path == path)
        .unwrap_or_else(|| panic!("read_files did not return {path}"))
}

#[test]
fn a_stale_citation_in_a_toml_file_is_reported() {
    let mut files = read_files().unwrap();
    let stale = format!("# See ADR-{:04}.\n", 9999);
    file_at(&mut files, "Cargo.toml").contents.push_str(&stale);

    assert_eq!(
        check::run(Selection::Docs, &files),
        [Violation::UnknownRecord {
            path: "Cargo.toml".to_owned(),
            number: 9999,
        }]
    );
}

#[test]
fn a_node_link_to_a_tracked_json_or_toml_file_is_not_broken() {
    let mut files = read_files().unwrap();
    let links = "\n[A schema](../../crates/crumbkit/schemas/usage-v1.json) \
                 and [the lint settings](../../clippy.toml).\n";
    file_at(&mut files, "docs/decisions/README.md")
        .contents
        .push_str(links);

    assert_eq!(check::run(Selection::Docs, &files), []);
}
