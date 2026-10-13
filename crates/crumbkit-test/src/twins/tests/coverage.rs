//! Tests for `check_twins`, which runs every twin of a tool and requires a
//! twin for each declared code.

use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};

use crate::examples::Examples;
use crate::recoveries::{Failing, Unprovoked};
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::{Twin, Twins, check_twins};

/// The code of a book that does not exist.
const NOT_FOUND: Code = Code::new("tool", "books", "not_found");
/// The code of a list of ids that cannot be read.
const UNREADABLE: Code = Code::new("tool", "books", "unreadable");
/// The code of a book file that disappeared, which no command line provokes.
const REMOVED: Code = Code::new("tool", "books", "removed_file");

/// The tool's declared codes.
const CODES: &[&[&Code]] = &[&[&NOT_FOUND, &UNREADABLE, &REMOVED]];

/// The script's answers: `books show 7` fails with `NOT_FOUND`, `books
/// list --limit 0` fails with crumbkit's usage code, `books show 8` fails
/// with `UNREADABLE`, and `books show 2` and `books list --limit 1` exit with
/// code 0, with or without `--json` at the end. Any other command line
/// fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'books show 7') printf 'error: no book 7\n  code:  tool::books::not_found\n' >&2; exit 1;;
  'books show 8') printf 'error: unreadable\n  code:  tool::books::unreadable\n' >&2; exit 1;;
  'books list --limit 0') printf 'error: 0 is not a limit\n  code:  crumbkit::cli::usage\n' >&2; exit 2;;
  'books show 2'|'books list --limit 1') echo shown; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// The twin of `NOT_FOUND`: book 7 does not exist, and book 2 does.
const SHOW_7: Twin<'_> = Twin::new(
    Failing::new(&NOT_FOUND, &["books", "show", "7"]),
    &["books", "show", "2"],
);

/// The twin of crumbkit's usage code, which needs no entry in `CODES`.
const LIMIT_0: Twin<'_> = Twin::new(
    Failing::new(
        &crumbkit::codes::CLI_USAGE,
        &["books", "list", "--limit", "0"],
    ),
    &["books", "list", "--limit", "1"],
);

/// Returns the panic message of `check_twins` on the tool, with `pairs` and
/// the listed codes `unprovoked`, or `None` when it passes.
fn checked(pairs: &[Twin<'_>], unprovoked: &[Unprovoked<'_>]) -> Option<String> {
    let dir = TempDir::new("twins-coverage").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let twins = Twins::new(Examples::new(&path, ""), Spelling::COLONS, CODES)
        .pairs(pairs)
        .unprovoked(unprovoked);
    panic_message(|| check_twins(&Guide::new(), &twins))
}

#[test]
fn a_declared_code_without_a_twin_or_an_unprovoked_entry_fails_the_twin_check() {
    let unprovoked = [Unprovoked::new(&REMOVED, "Only a full disk provokes it.")];

    assert_eq!(
        checked(&[SHOW_7, LIMIT_0], &unprovoked).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::unreadable: no twin refuses the declared code, and Twins::unprovoked does not list it"
        )
    );
}

#[test]
fn an_unprovoked_entry_for_a_code_with_a_twin_or_an_undeclared_code_fails_the_twin_check() {
    const STRAY: Code = Code::new("tool", "stray", "gone");
    let unprovoked = [
        Unprovoked::new(&NOT_FOUND, "A missing book needs a full disk."),
        Unprovoked::new(&UNREADABLE, "Only a broken file provokes it."),
        Unprovoked::new(&REMOVED, "Only a full disk provokes it."),
        Unprovoked::new(&STRAY, "Nothing provokes it."),
    ];

    assert_eq!(
        checked(&[SHOW_7], &unprovoked).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::not_found: Twins::unprovoked lists the code, and a twin refuses it\n\
             tool::stray::gone: Twins::unprovoked lists the code, and it is not a declared code"
        )
    );
}

#[test]
fn every_broken_pair_is_reported_and_the_check_does_not_stop_at_the_first() {
    const STRAY: Code = Code::new("tool", "stray", "gone");
    let pairs = [
        Twin::new(
            Failing::new(&NOT_FOUND, &["books", "show", "7"]),
            &["books", "show", "8"],
        ),
        Twin::new(
            Failing::new(&UNREADABLE, &["books", "show", "2"]),
            &["books", "show", "8"],
        ),
        Twin::new(
            Failing::new(&STRAY, &["books", "show", "9"]),
            &["books", "show", "2"],
        ),
    ];
    let unprovoked = [Unprovoked::new(&REMOVED, "Only a full disk provokes it.")];

    assert_eq!(
        checked(&pairs, &unprovoked).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the admitted line `tool books show 8 --json` must exit with code 0, and it ends with exit status: 1 after writing this to standard error:\n\
             error: unreadable\n  code:  tool::books::unreadable\n\n\
             the refused line `tool books show 2 --json` must fail with tool::books::unreadable, and it exits with code 0\n\
             the refused line `tool books show 9` fails with tool::stray::gone, which is not a declared code or one of crumbkit's codes"
        )
    );
}

/// Returns the panic message of `check_twins` on the tool `script`, which
/// declares `NOT_FOUND` alone, with `pairs`, or `None` when it passes.
fn checked_with(script: &str, pairs: &[Twin<'_>]) -> Option<String> {
    let dir = TempDir::new("twins-json").unwrap();
    dir.write_script("tool", script);
    let path = dir.path().join("tool").display().to_string();
    let twins =
        Twins::new(Examples::new(&path, ""), Spelling::COLONS, &[&[&NOT_FOUND]]).pairs(pairs);
    panic_message(|| check_twins(&Guide::new(), &twins))
}

#[test]
fn check_twins_adds_json_to_both_lines_of_a_twin_that_lacks_it() {
    // The tool answers only command lines that end with `--json`.
    const JSON_ONLY: &str = r#"#!/bin/sh
case "$*" in
  'books show 7 --json') printf '%s\n' '{"schema":"crumbkit/failure/v1","code":"tool::books::not_found","message":"m","recovery":{"kind":"retry"}}'; exit 1;;
  'books show 2 --json') echo '{}'; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

    assert_eq!(checked_with(JSON_ONLY, &[SHOW_7]), None);
}

#[test]
fn check_twins_runs_a_twin_whose_changed_word_is_json_as_written() {
    // `books show 7` fails only with `--json`, so a check that added
    // `--json` to the admitted line would see it fail.
    const JSON_REFUSED: &str = r#"#!/bin/sh
case "$*" in
  'books show 7 --json') printf 'error: no JSON\n  code:  tool::books::not_found\n' >&2; exit 1;;
  'books show 7') echo shown; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;
    let pair = Twin::new(
        Failing::new(&NOT_FOUND, &["books", "show", "7", "--json"]),
        &["books", "show", "7"],
    );

    assert_eq!(checked_with(JSON_REFUSED, &[pair]), None);
}
