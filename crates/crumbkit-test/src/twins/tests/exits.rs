//! Tests for the exit codes that the twin checks compare with the exits
//! that the guide declares for failure codes (ADR-0037).

use crumbkit::help::{Exit, Guide};
use crumbkit::{Code, Spelling};

use super::tree;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::{Twin, Twins, check_twin, check_twins};

/// The code of a download that ran out of time.
const TIMED_OUT: Code = Code::new("tool", "books", "timed_out");
/// The code of a book that does not exist.
const NOT_FOUND: Code = Code::new("tool", "books", "not_found");

/// A guide that lists `TIMED_OUT` for the exit code 124.
const GUIDE: Guide =
    Guide::new().exits(&[Exit::new(124, "The download ran out of time.").failures(&[&TIMED_OUT])]);

/// The script's answers: `books get 1` fails with `TIMED_OUT` and code
/// 124, `books get 2` fails with `TIMED_OUT` and code 1, `books show 7`
/// fails with `NOT_FOUND` and code 124, `books show 8` fails with
/// `NOT_FOUND` and code 1, and `books get 3` and `books show 2` exit with
/// code 0, with or without `--json` at the end.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'books get 1') printf 'error: slow\n  code:  tool::books::timed_out\n' >&2; exit 124;;
  'books get 2') printf 'error: slow\n  code:  tool::books::timed_out\n' >&2; exit 1;;
  'books show 7') printf 'error: no book 7\n  code:  tool::books::not_found\n' >&2; exit 124;;
  'books show 8') printf 'error: no book 8\n  code:  tool::books::not_found\n' >&2; exit 1;;
  'books get 3'|'books show 2') echo shown; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// Runs `check_twin` on the script with `GUIDE`, refusing `refused` with
/// `code` and admitting the same command with the book id `3` or `2`, and
/// returns its panic message, or `None` when it passes.
fn checked(code: &'static Code, refused: &'static [&'static str]) -> Option<String> {
    let admitted: &[&str] = if refused[1] == "get" {
        &["books", "get", "3"]
    } else {
        &["books", "show", "2"]
    };
    let dir = TempDir::new("twins-exits").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let examples = Examples::new(&path, "");
    panic_message(|| {
        check_twin(
            &tree(),
            &GUIDE,
            &examples,
            Spelling::COLONS,
            &Failing::new(code, refused),
            admitted,
        );
    })
}

#[test]
fn a_refused_line_that_exits_with_the_code_that_lists_its_failure_passes() {
    assert_eq!(checked(&TIMED_OUT, &["books", "get", "1"]), None);
}

#[test]
fn a_refused_line_whose_failure_an_exit_lists_and_that_exits_with_another_code_fails_the_check() {
    assert_eq!(
        checked(&TIMED_OUT, &["books", "get", "2"]).unwrap(),
        "the refused line `tool books get 2` fails with tool::books::timed_out, which the guide's exit code 124 lists, and it ends with exit status: 1"
    );
}

#[test]
fn a_refused_line_whose_failure_no_exit_lists_passes_with_any_code_that_lists_no_failure() {
    assert_eq!(checked(&NOT_FOUND, &["books", "show", "8"]), None);
}

#[test]
fn a_refused_line_whose_failure_no_exit_lists_fails_the_check_when_it_exits_with_a_listing_code() {
    assert_eq!(
        checked(&NOT_FOUND, &["books", "show", "7"]).unwrap(),
        "the refused line `tool books show 7` fails with tool::books::not_found, which no exit code of the guide lists, and it exits with 124, which the guide declares for the failure codes that it lists"
    );
}

#[test]
fn a_failure_code_that_an_exit_lists_and_no_codes_list_holds_fails_check_twins() {
    const GONE: Code = Code::new("tool", "books", "gone");
    const LISTING: Guide = Guide::new().exits(&[
        Exit::new(124, "Slow.").failures(&[&TIMED_OUT]),
        Exit::new(3, "Gone.").failures(&[&GONE]),
    ]);
    const CODES: &[&[&Code]] = &[&[&TIMED_OUT]];
    const PAIRS: &[Twin<'_>] = &[Twin::new(
        Failing::new(&TIMED_OUT, &["books", "get", "1"]),
        &["books", "get", "3"],
    )];
    let dir = TempDir::new("twins-exits-codes").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let twins = Twins::new(Examples::new(&path, ""), Spelling::COLONS, CODES).pairs(PAIRS);

    assert_eq!(
        panic_message(|| check_twins(&tree(), &LISTING, &twins)).unwrap(),
        "the tool's twins break crumbkit's twin rule:\n\
         tool::books::gone: the guide's exit code 3 lists the failure code, and no CODES list holds it"
    );
}
