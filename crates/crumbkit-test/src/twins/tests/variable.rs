//! Tests for twins that differ by one environment variable, and for the
//! variables of a failing line.

use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};

use super::tree;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::{Twin, Twins, check_twin, check_twins};

/// The code of a listing that goes deeper than the tool allows.
const TOO_DEEP: Code = Code::new("tool", "books", "too_deep");
/// The code of a listing without a shelf.
const NO_SHELF: Code = Code::new("tool", "books", "no_shelf");
/// The code of a book that does not exist.
const NOT_FOUND: Code = Code::new("tool", "books", "not_found");

/// The script's answers: `books list` fails with `TOO_DEEP` when `DEPTH` is
/// 1, fails with `NO_SHELF` when `BOOKS` is not set, and succeeds
/// otherwise. `books stuck` always fails with `TOO_DEEP`. Both answer with
/// or without `--json` at the end. Any other command line fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'books list')
    if [ "$DEPTH" = 1 ]; then printf 'error: too deep\n  code:  tool::books::too_deep\n' >&2; exit 1; fi
    if [ -z "${BOOKS+x}" ]; then printf 'error: no shelf\n  code:  tool::books::no_shelf\n' >&2; exit 1; fi
    echo listed; exit 0;;
  'books stuck') printf 'error: too deep\n  code:  tool::books::too_deep\n' >&2; exit 1;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// The variable twin of `NO_SHELF`: the line without `BOOKS` fails, and the
/// line with the `BOOKS` of `Examples::env` succeeds.
const NO_SHELF_TWIN: Twin<'_> = Twin::variable(
    Failing::new(&NO_SHELF, &["books", "list"]).env(&[("BOOKS", None)]),
    "BOOKS",
);

/// Returns the panic message of `check_twins` on the script, with
/// `BOOKS=/b` in `Examples::env` and `pairs`, or `None` when it passes.
fn checked(pairs: &[Twin<'_>]) -> Option<String> {
    let dir = TempDir::new("twins-variable").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let examples = Examples::new(&path, "").env(&[("BOOKS", "/b")]);
    let twins = Twins::new(examples, Spelling::COLONS, &[&[&TOO_DEEP, &NO_SHELF]]).pairs(pairs);
    panic_message(|| check_twins(&tree(), &Guide::new(), &twins))
}

#[test]
fn a_variable_twin_passes_when_the_refused_line_fails_with_its_variable_and_the_line_without_it_succeeds()
 {
    let too_deep = Twin::variable(
        Failing::new(&TOO_DEEP, &["books", "list"]).env(&[("DEPTH", Some("1"))]),
        "DEPTH",
    );

    assert_eq!(checked(&[too_deep, NO_SHELF_TWIN]), None);
}

#[test]
fn a_variable_twin_whose_line_without_the_variable_still_fails_fails_the_check() {
    let stuck = Twin::variable(
        Failing::new(&TOO_DEEP, &["books", "stuck"]).env(&[("DEPTH", Some("1"))]),
        "DEPTH",
    );

    assert_eq!(
        checked(&[stuck, NO_SHELF_TWIN]).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the admitted line `tool books stuck --json` (without DEPTH=1) must exit with code 0, and it ends with exit status: 1 after writing this to standard error:\n\
             error: too deep\n  code:  tool::books::too_deep\n"
        )
    );
}

#[test]
fn a_variable_twin_that_names_a_variable_the_refused_line_does_not_set_fails_the_check() {
    let width = Twin::variable(
        Failing::new(&TOO_DEEP, &["books", "list"]).env(&[("DEPTH", Some("1"))]),
        "WIDTH",
    );

    assert_eq!(
        checked(&[width, NO_SHELF_TWIN]).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the variable twin of the refused line `tool books list --json` (with DEPTH=1) names WIDTH, which the refused line neither sets nor removes"
        )
    );
}

#[test]
fn a_failing_line_s_variable_wins_over_examples_env_and_the_check_s_xdg_directories() {
    // The tool refuses book 7 only with the line's own variables, and shows
    // book 2 only with the line's `BOOKS`, so both lines run with them.
    const PRECEDENCE: &str = r#"#!/bin/sh
case "$*" in
  'books show 7')
    if [ "$BOOKS" = b ] && [ "$XDG_CONFIG_HOME" = /nowhere ]; then printf 'error: no book 7\n  code:  tool::books::not_found\n' >&2; exit 1; fi
    echo "BOOKS=$BOOKS XDG_CONFIG_HOME=$XDG_CONFIG_HOME" >&2; exit 3;;
  'books show 2') if [ "$BOOKS" = b ]; then exit 0; fi; exit 3;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;
    let dir = TempDir::new("twins-variable-wins").unwrap();
    dir.write_script("tool", PRECEDENCE);
    let path = dir.path().join("tool").display().to_string();
    let examples = Examples::new(&path, "").env(&[("BOOKS", "a")]);
    let refused = Failing::new(&NOT_FOUND, &["books", "show", "7"])
        .env(&[("BOOKS", Some("b")), ("XDG_CONFIG_HOME", Some("/nowhere"))]);

    let output = check_twin(
        &tree(),
        &Guide::new(),
        &examples,
        Spelling::COLONS,
        &refused,
        &["books", "show", "2"],
    );

    assert_eq!(output.status.code(), Some(1));
}
