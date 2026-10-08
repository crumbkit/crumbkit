//! Tests for the command line and for the repository itself.

use super::{parse_args, read_files};
use crate::check::{self, Selection};

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

#[test]
fn check_alone_selects_every_check() {
    assert_eq!(parse_args(&args(&["check"])), Some(Selection::All));
}

#[test]
fn check_docs_selects_only_the_docs_check() {
    assert_eq!(parse_args(&args(&["check", "docs"])), Some(Selection::Docs));
}

#[test]
fn any_other_command_line_is_refused() {
    assert_eq!(parse_args(&args(&[])), None);
    assert_eq!(parse_args(&args(&["docs"])), None);
    assert_eq!(parse_args(&args(&["check", "rust"])), None);
    assert_eq!(parse_args(&args(&["check", "docs", "extra"])), None);
}

#[test]
fn the_repository_passes_every_check() {
    let files = read_files().unwrap();

    assert_eq!(check::run(Selection::All, &files), []);
}
