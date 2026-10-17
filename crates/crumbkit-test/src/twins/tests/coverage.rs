//! Tests for `check_twins`, which runs every twin of a tool and requires a
//! twin for each declared code.

use crumbkit::help::{Guide, Leaf, Rename};
use crumbkit::{Code, Spelling};

use super::tree;
use crate::allow::Allow;
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
/// with `UNREADABLE`, `books show 5` fails with crumbkit-child's code
/// `tool::child::no_json`, and `books show 2` and `books list --limit 1` exit with
/// code 0, with or without `--json` at the end. Any other command line
/// fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'books show 7') printf 'error: no book 7\n  code:  tool::books::not_found\n' >&2; exit 1;;
  'books show 8') printf 'error: unreadable\n  code:  tool::books::unreadable\n' >&2; exit 1;;
  'books list --limit 0') printf 'error: 0 is not a limit\n  code:  tool::cli::usage\n' >&2; exit 2;;
  'books show 5') printf 'error: the viewer prints no JSON\n  code:  tool::child::no_json\n' >&2; exit 2;;
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
    checked_allowing(pairs, unprovoked, &[])
}

/// Returns the panic message of `check_twins` on the tool, with `pairs`,
/// the listed codes `unprovoked` and the opt-outs `allow`, or `None` when
/// it passes.
fn checked_allowing(
    pairs: &[Twin<'_>],
    unprovoked: &[Unprovoked<'_>],
    allow: &[Allow],
) -> Option<String> {
    let dir = TempDir::new("twins-coverage").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let twins = Twins::new(
        Examples::new(&path, "").allow(allow),
        Spelling::COLONS,
        CODES,
    )
    .pairs(pairs)
    .unprovoked(unprovoked);
    panic_message(|| check_twins(&tree(), &Guide::new(), &twins))
}

#[test]
fn a_declared_code_without_a_twin_or_an_unprovoked_entry_fails_the_twin_check() {
    let unprovoked = [Unprovoked::new(&REMOVED, "Only a full disk provokes it.")];

    assert_eq!(
        checked(&[SHOW_7, LIMIT_0], &unprovoked).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::unreadable: no twin refuses the declared code, Twins::unprovoked does not list it, and no opt-out made with Allow::untwinned names it"
        )
    );
}

#[test]
fn a_twin_of_a_code_in_the_crumbkit_component_that_crumbkit_does_not_list_needs_no_declaration() {
    const NO_JSON: Code = Code::new("crumbkit", "child", "no_json");
    let no_json = Twin::new(
        Failing::new(&NO_JSON, &["books", "show", "5"]),
        &["books", "show", "2"],
    );
    let unprovoked = [
        Unprovoked::new(&UNREADABLE, "Only a broken file provokes it."),
        Unprovoked::new(&REMOVED, "Only a full disk provokes it."),
    ];

    assert_eq!(checked(&[SHOW_7, no_json], &unprovoked), None);
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
    panic_message(|| check_twins(&tree(), &Guide::new(), &twins))
}

#[test]
fn check_twins_adds_json_to_both_lines_of_a_twin_that_lacks_it() {
    // The tool answers only command lines that end with `--json`.
    const JSON_ONLY: &str = r#"#!/bin/sh
case "$*" in
  'books show 7 --json') printf '%s\n' '{"schema":"tool/failure/v1","code":"tool::books::not_found","message":"m","recovery":{"kind":"retry"}}'; exit 1;;
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

/// The reason of each untwinned opt-out in these tests.
const BROKEN_FILE: &str = "A broken file causes it, and no single word does.";

/// The listed code that no command line provokes.
const REMOVED_ENTRY: Unprovoked<'_> = Unprovoked::new(&REMOVED, "Only a full disk provokes it.");

#[test]
fn a_declared_code_without_a_twin_passes_when_an_untwinned_opt_out_names_a_line_that_fails_with_it()
{
    const ALLOW: &[Allow] = &[Allow::untwinned(
        Failing::new(&UNREADABLE, &["books", "show", "8"]),
        BROKEN_FILE,
    )];

    assert_eq!(checked_allowing(&[SHOW_7], &[REMOVED_ENTRY], ALLOW), None);
}

#[test]
fn an_untwinned_opt_out_whose_code_a_twin_refuses_or_twins_unprovoked_lists_or_crumbkit_owns_is_stale()
 {
    const ALLOW: &[Allow] = &[
        Allow::untwinned(
            Failing::new(&NOT_FOUND, &["books", "show", "7"]),
            BROKEN_FILE,
        ),
        Allow::untwinned(Failing::new(&REMOVED, &["books", "show", "8"]), BROKEN_FILE),
        Allow::untwinned(
            Failing::new(
                &crumbkit::codes::CLI_USAGE,
                &["books", "list", "--limit", "0"],
            ),
            BROKEN_FILE,
        ),
        Allow::untwinned(
            Failing::new(&UNREADABLE, &["books", "show", "8"]),
            BROKEN_FILE,
        ),
    ];

    assert_eq!(
        checked_allowing(&[SHOW_7], &[REMOVED_ENTRY], ALLOW).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the opt-out from `twins/no-twin` for `tool books show 7` names tool::books::not_found, which a twin refuses, so it is stale\n\
             the opt-out from `twins/no-twin` for `tool books show 8` names tool::books::removed_file, which Twins::unprovoked lists, so it is stale\n\
             the opt-out from `twins/no-twin` for `tool books list --limit 0` names tool::cli::usage, one of crumbkit's codes, which need no twin, so it is stale"
        )
    );
}

#[test]
fn an_untwinned_opt_out_whose_line_succeeds_or_fails_with_another_code_fails_the_check() {
    const ALLOW: &[Allow] = &[
        Allow::untwinned(
            Failing::new(&UNREADABLE, &["books", "show", "2"]),
            BROKEN_FILE,
        ),
        Allow::untwinned(
            Failing::new(&UNREADABLE, &["books", "show", "7"]),
            BROKEN_FILE,
        ),
    ];

    assert_eq!(
        checked_allowing(&[SHOW_7], &[REMOVED_ENTRY], ALLOW).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the opt-out from `twins/no-twin` for `tool books show 2`: the refused line `tool books show 2 --json` must fail with tool::books::unreadable, and it exits with code 0\n\
             the opt-out from `twins/no-twin` for `tool books show 7`: the refused line `tool books show 7 --json` must fail with tool::books::unreadable, and it fails with tool::books::not_found"
        )
    );
}

#[test]
fn a_declared_code_that_the_guide_prints_in_place_of_one_of_crumbkits_codes_needs_no_twin() {
    const BAD_LIMIT: Code = Code::new("tool", "books", "bad_limit");
    const GUIDE: Guide =
        Guide::new().codes(&[Rename::new(&crumbkit::codes::CLI_USAGE, &BAD_LIMIT)]);
    const CODES: &[&[&Code]] = &[&[&NOT_FOUND, &BAD_LIMIT]];
    let dir = TempDir::new("twins-renamed").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let twins = Twins::new(Examples::new(&path, ""), Spelling::COLONS, CODES).pairs(&[SHOW_7]);

    let problems = panic_message(|| check_twins(&tree(), &GUIDE, &twins));

    assert_eq!(problems, None);
}

#[test]
fn a_declared_code_that_a_leaf_prints_in_place_of_one_of_crumbkits_codes_needs_no_twin() {
    const STALE: Code = Code::new("tool", "books", "stale");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "show"])
        .codes(&[Rename::new(&crumbkit::codes::VERSION_CONFLICT, &STALE)])]);
    const CODES: &[&[&Code]] = &[&[&NOT_FOUND, &STALE]];
    let dir = TempDir::new("twins-leaf-renamed").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let twins = Twins::new(Examples::new(&path, ""), Spelling::COLONS, CODES).pairs(&[SHOW_7]);

    let problems = panic_message(|| check_twins(&tree(), &GUIDE, &twins));

    assert_eq!(problems, None);
}

#[test]
fn a_twin_check_built_from_owned_codes_requires_a_twin_for_each_code_it_holds() {
    let dir = TempDir::new("twins-owned-codes").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let codes: Vec<Code> = vec![NOT_FOUND, UNREADABLE, REMOVED];
    let unprovoked = [Unprovoked::from_code(
        codes[2],
        "Only a book file that disappears provokes it.",
    )];
    let twins = Twins::from_codes(Examples::new(&path, ""), Spelling::COLONS, &codes)
        .pairs(&[SHOW_7])
        .unprovoked(&unprovoked);

    let problems = panic_message(|| check_twins(&tree(), &Guide::new(), &twins));

    assert_eq!(
        problems.as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::unreadable: no twin refuses the declared code, Twins::unprovoked does not list it, and no opt-out made with Allow::untwinned names it"
        )
    );
}
