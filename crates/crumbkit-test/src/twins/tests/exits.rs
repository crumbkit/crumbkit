//! Tests for the exit codes that the twin checks compare with the exits
//! that the tool record declares for failure codes (ADR-0037).

use crumbkit::help::{Help, Leaf};
use crumbkit::tool::Exit;
use crumbkit::{Code, Tool};

use super::tree;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::{Twin, Twins, check_twin, check_twins};
use crumbkit::tool::Failures;

/// The code of a download that ran out of time.
const TIMED_OUT: Code = Code::new("tool", "books", "timed_out");
/// The code of a book that does not exist.
const NOT_FOUND: Code = Code::new("tool", "books", "not_found");

/// The tool's code of a downloader that it ran and that failed.
const DOWNLOAD_FAILED: Code = Code::new("tool", "books", "download_failed");

/// The exit code 124, which `TIMED_OUT` ends a run with.
const EXITS: &[Exit] = &[Exit::new(124, "The download ran out of time.").failures(&[&TIMED_OUT])];

/// A tool record that lists `TIMED_OUT` for the exit code 124.
const TOOL: Tool = Tool::new().failures(Failures::new().exits(EXITS));

/// `TOOL` with a leaf that declares that `books get` passes on the exit
/// code of the downloader that it runs.
const PASSING: Tool = Tool::new()
    .help(Help::new().leaves(&[Leaf::new(&["books", "get"]).passes_on_exit("the downloader")]))
    .failures(Failures::new().exits(EXITS));

/// The script's answers: `books get 1` fails with `TIMED_OUT` and code
/// 124, `books get 2` fails with `TIMED_OUT` and code 1, `books show 7`
/// fails with `NOT_FOUND` and code 124, `books show 8` fails with
/// `NOT_FOUND` and code 1, `books show 9` fails with `NOT_FOUND` and code
/// 5, `books get 9` fails with `tool::cli::usage` and code 64,
/// `books get --limit x` fails with crumbkit's usage code and code 1,
/// `books get --limit y` fails with it and code 2, `books get 4` fails with
/// `DOWNLOAD_FAILED` and code 7, `books get 5` fails with `NOT_FOUND` and code
/// 124, and `books get 3`, `books get --limit 3` and `books show 2` exit
/// with code 0, with or without `--json` at the end.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'books get 1') printf 'error: slow\n  code:  tool::books::timed_out\n' >&2; exit 124;;
  'books get 2') printf 'error: slow\n  code:  tool::books::timed_out\n' >&2; exit 1;;
  'books show 7') printf 'error: no book 7\n  code:  tool::books::not_found\n' >&2; exit 124;;
  'books show 8') printf 'error: no book 8\n  code:  tool::books::not_found\n' >&2; exit 1;;
  'books show 9') printf 'error: no book 9\n  code:  tool::books::not_found\n' >&2; exit 5;;
  'books get 9') printf 'error: refused\n  code:  tool::cli::usage\n' >&2; exit 64;;
  'books get --limit x') printf 'error: x is no limit\n  code:  tool::cli::usage\n' >&2; exit 1;;
  'books get --limit y') printf 'error: y is no limit\n  code:  tool::cli::usage\n' >&2; exit 2;;
  'books get 4') printf 'error: the downloader failed\n  code:  tool::books::download_failed\n' >&2; exit 7;;
  'books get 5') printf 'error: no book 5\n  code:  tool::books::not_found\n' >&2; exit 124;;
  'books get 3'|'books get --limit 3'|'books show 2') echo shown; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// Runs `check_twin` on the script with `TOOL`, refusing `refused` with
/// `code` and admitting the same line with `3` in place of its last word,
/// or `books show 2` for `books show`, and returns its panic message, or
/// `None` when it passes.
fn checked(code: &'static Code, refused: &'static [&'static str]) -> Option<String> {
    checked_with(&TOOL, code, refused)
}

/// Runs `check_twin` as [`checked`] does, with `tool`.
fn checked_with(
    tool: &Tool,
    code: &'static Code,
    refused: &'static [&'static str],
) -> Option<String> {
    let mut admitted: Vec<&str> = refused.to_vec();
    if let Some(last) = admitted.last_mut() {
        *last = if refused[1] == "get" { "3" } else { "2" };
    }
    let dir = TempDir::new("twins-exits").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let examples = Examples::new(&path, "");
    panic_message(|| {
        check_twin(
            &tree(),
            tool,
            &examples,
            &Failing::new(code, refused),
            &admitted,
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
        "the refused line `tool books get 2` fails with tool::books::timed_out, which the tool record's exit code 124 lists, and it ends with exit status: 1"
    );
}

#[test]
fn a_refused_line_whose_failure_no_exit_lists_passes_when_it_exits_with_code_1() {
    assert_eq!(checked(&NOT_FOUND, &["books", "show", "8"]), None);
}

#[test]
fn a_refused_line_whose_failure_no_exit_lists_fails_the_check_when_it_exits_with_another_code() {
    assert_eq!(
        checked(&NOT_FOUND, &["books", "show", "9"]).unwrap(),
        "the refused line `tool books show 9` fails with tool::books::not_found, which no exit code of the tool record lists, and it ends with exit status: 5, where such a failure exits with code 1. A command that passes on the exit code of a program it runs declares it with `Leaf::passes_on_exit`"
    );
}

#[test]
fn a_usage_refusal_that_exits_with_code_2_passes() {
    assert_eq!(
        checked(
            &crumbkit::codes::CLI_USAGE,
            &["books", "get", "--limit", "y"]
        ),
        None
    );
}

#[test]
fn a_usage_refusal_that_exits_with_another_code_than_2_fails_the_check_on_a_command_that_passes_on_exit_codes()
 {
    assert_eq!(
        checked_with(
            &PASSING,
            &crumbkit::codes::CLI_USAGE,
            &["books", "get", "--limit", "x"]
        )
        .unwrap(),
        "the refused line `tool books get --limit x` fails with tool::cli::usage, which refuses the command line, and it ends with exit status: 1, where the tool record gives a refused command line the exit code 2"
    );
}

#[test]
fn a_failure_of_a_command_that_passes_on_a_program_s_exit_code_may_end_with_that_code() {
    assert_eq!(
        checked_with(&PASSING, &DOWNLOAD_FAILED, &["books", "get", "4"]),
        None
    );
}

#[test]
fn a_failed_child_fails_the_check_when_its_command_does_not_declare_that_it_passes_on_exit_codes() {
    assert_eq!(
        checked(&DOWNLOAD_FAILED, &["books", "get", "4"]).unwrap(),
        "the refused line `tool books get 4` fails with tool::books::download_failed, which no exit code of the tool record lists, and it ends with exit status: 7, where such a failure exits with code 1. A command that passes on the exit code of a program it runs declares it with `Leaf::passes_on_exit`"
    );
}

#[test]
fn a_command_that_passes_on_exit_codes_fails_the_check_when_an_unlisted_failure_exits_with_a_listing_code()
 {
    assert_eq!(
        checked_with(&PASSING, &NOT_FOUND, &["books", "get", "5"]).unwrap(),
        "the refused line `tool books get 5` fails with tool::books::not_found, which no exit code of the tool record lists, and it exits with 124, which the tool record declares for the failure codes that it lists"
    );
}

#[test]
fn a_refused_line_whose_failure_no_exit_lists_fails_the_check_when_it_exits_with_a_listing_code() {
    assert_eq!(
        checked(&NOT_FOUND, &["books", "show", "7"]).unwrap(),
        "the refused line `tool books show 7` fails with tool::books::not_found, which no exit code of the tool record lists, and it exits with 124, which the tool record declares for the failure codes that it lists"
    );
}

#[test]
fn a_failure_code_that_an_exit_lists_and_no_codes_list_holds_fails_check_twins() {
    const GONE: Code = Code::new("tool", "books", "gone");
    const LISTING: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(124, "Slow.").failures(&[&TIMED_OUT]),
        Exit::new(3, "Gone.").failures(&[&GONE]),
    ]));
    const CODES: &[&[&Code]] = &[&[&TIMED_OUT]];
    const PAIRS: &[Twin<'_>] = &[Twin::new(
        Failing::new(&TIMED_OUT, &["books", "get", "1"]),
        &["books", "get", "3"],
    )];
    let dir = TempDir::new("twins-exits-codes").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let twins = Twins::new(Examples::new(&path, ""), CODES).pairs(PAIRS);

    assert_eq!(
        panic_message(|| check_twins(&tree(), &LISTING, &twins)).unwrap(),
        "the tool's twins break crumbkit's twin rule:\n\
         tool::books::gone: the tool record's exit code 3 lists the failure code, and no CODES list holds it"
    );
}
