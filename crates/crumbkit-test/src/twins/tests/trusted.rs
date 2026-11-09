//! Tests for the codes that `check_twins` trusts: a code is crumbkit's only
//! when `crumbkit::codes::ALL` or the list of the add-on crates' codes holds
//! it, whatever its component (ADR-0177).

use crumbkit::tool::{Exit, Rename};
use crumbkit::{Code, Tool};

use super::tree;
use crate::allow::Allow;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::{Twin, Twins, check_twins};
use crumbkit::tool::Failures;

/// The code of a book that does not exist.
const NOT_FOUND: Code = Code::new("tool", "books", "not_found");
/// A code in the `crumbkit` component that no crumbkit crate lists.
const UNLISTED: Code = Code::new("crumbkit", "books", "unlisted");
/// The tool's code that a tool record prints in place of another code.
const RENAMED: Code = Code::new("tool", "books", "renamed");

/// The script's answers: `books show 7` fails with `NOT_FOUND`, `books show
/// 5` fails with crumbkit-child's code `tool::child::not_found`, `books
/// show 4` fails with crumbkit-aliases's code `tool::aliases::unreadable`,
/// `books show 6` fails with `UNLISTED`, which prints with the tool's name,
/// and `books show 2` exits with code 0, with or without `--json` at the
/// end. Any other command line fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'books show 7') printf 'error: no book 7\n  code:  tool::books::not_found\n' >&2; exit 1;;
  'books show 5') printf 'error: cannot run `viewer`: no such program\n  code:  tool::child::not_found\n' >&2; exit 1;;
  'books show 4') printf 'error: the aliases file is unreadable\n  code:  tool::aliases::unreadable\n' >&2; exit 1;;
  'books show 6') printf 'error: unlisted\n  code:  tool::books::unlisted\n' >&2; exit 1;;
  'books show 2') echo shown; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// The twin of `NOT_FOUND`: book 7 does not exist, and book 2 does.
const SHOW_7: Twin<'_> = Twin::new(
    Failing::new(&NOT_FOUND, &["books", "show", "7"]),
    &["books", "show", "2"],
);

/// The twin of `UNLISTED`: book 6 fails with it, and book 2 does not.
const SHOW_6: Twin<'_> = Twin::new(
    Failing::new(&UNLISTED, &["books", "show", "6"]),
    &["books", "show", "2"],
);

/// The twin of crumbkit-child's code that names a missing program.
const SHOW_5: Twin<'_> = Twin::new(
    Failing::new(&crumbkit_child::codes::NOT_FOUND, &["books", "show", "5"]),
    &["books", "show", "2"],
);

/// The twin of crumbkit-aliases's code of an unreadable file.
const SHOW_4: Twin<'_> = Twin::new(
    Failing::new(
        &crumbkit_aliases::codes::UNREADABLE,
        &["books", "show", "4"],
    ),
    &["books", "show", "2"],
);

/// Returns the panic message of `check_twins` on the tool with `codes`,
/// `pairs`, `tool` and the opt-outs `allow`, or `None` when it passes.
fn checked(codes: &[&[&Code]], pairs: &[Twin<'_>], tool: &Tool, allow: &[Allow]) -> Option<String> {
    let dir = TempDir::new("twins-trusted").unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let twins = Twins::new(Examples::new(&path, "").allow(allow), codes).pairs(pairs);
    panic_message(|| check_twins(&tree(), tool, &twins))
}

#[test]
fn a_twin_of_a_child_code_or_an_aliases_code_needs_no_declaration() {
    assert_eq!(
        checked(
            &[&[&NOT_FOUND]],
            &[SHOW_7, SHOW_5, SHOW_4],
            &Tool::new(),
            &[]
        ),
        None
    );
}

#[test]
fn a_declared_child_code_or_aliases_code_needs_no_twin() {
    let codes: &[&[&Code]] = &[&[
        &NOT_FOUND,
        &crumbkit_child::codes::NOT_STARTED,
        &crumbkit_aliases::codes::UNREADABLE,
    ]];

    assert_eq!(checked(codes, &[SHOW_7], &Tool::new(), &[]), None);
}

#[test]
fn a_declared_tool_code_renamed_from_a_child_code_needs_no_twin() {
    const TOOL: Tool = Tool::new().failures(
        Failures::new().codes(&[Rename::new(&crumbkit_child::codes::NOT_STARTED, &RENAMED)]),
    );

    assert_eq!(
        checked(&[&[&NOT_FOUND, &RENAMED]], &[SHOW_7], &TOOL, &[]),
        None
    );
}

#[test]
fn a_twin_that_refuses_a_crumbkit_component_code_on_neither_list_fails_as_not_declared() {
    assert_eq!(
        checked(&[&[&NOT_FOUND]], &[SHOW_7, SHOW_6], &Tool::new(), &[]).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the refused line `tool books show 6` fails with tool::books::unlisted, which is not a declared code or a code of crumbkit or of one of its add-on crates"
        )
    );
}

#[test]
fn a_declared_crumbkit_component_code_on_neither_list_needs_a_twin() {
    assert_eq!(
        checked(&[&[&NOT_FOUND, &UNLISTED]], &[SHOW_7], &Tool::new(), &[]).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::unlisted: no twin refuses the declared code, Twins::unprovoked does not list it, and no opt-out made with Allow::untwinned names it"
        )
    );
}

#[test]
fn a_declared_tool_code_renamed_from_a_crumbkit_component_code_on_neither_list_needs_a_twin() {
    const TOOL: Tool =
        Tool::new().failures(Failures::new().codes(&[Rename::new(&UNLISTED, &RENAMED)]));

    assert_eq!(
        checked(&[&[&NOT_FOUND, &RENAMED]], &[SHOW_7], &TOOL, &[]).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::renamed: no twin refuses the declared code, Twins::unprovoked does not list it, and no opt-out made with Allow::untwinned names it"
        )
    );
}

#[test]
fn an_exit_that_lists_a_crumbkit_component_code_on_neither_list_fails_the_twin_check() {
    const TOOL: Tool = Tool::new()
        .failures(Failures::new().exits(&[Exit::new(3, "Unlisted.").failures(&[&UNLISTED])]));

    assert_eq!(
        checked(&[&[&NOT_FOUND]], &[SHOW_7], &TOOL, &[]).as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::unlisted: the tool record's exit code 3 lists the failure code, and no CODES list holds it"
        )
    );
}

#[test]
fn an_untwinned_opt_out_for_a_declared_crumbkit_component_code_on_neither_list_is_accepted() {
    const ALLOW: &[Allow] = &[Allow::untwinned(
        Failing::new(&UNLISTED, &["books", "show", "6"]),
        "A broken file causes it, and no single word does.",
    )];

    assert_eq!(
        checked(&[&[&NOT_FOUND, &UNLISTED]], &[SHOW_7], &Tool::new(), ALLOW),
        None
    );
}
