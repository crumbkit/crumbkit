//! Tests for the lists of library codes that `check_twins` trusts: a code
//! is the library's only when one of the lists holds it, whatever its
//! component (ADR-0177).

use crumbkit::Code;
use crumbkit::help::{Exit, Guide, Rename};

use super::{TRUSTED, tree};
use crate::allow::Allow;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::{Twin, Twins, check_twins};

/// The code of a book that does not exist.
const NOT_FOUND: Code = Code::new("tool", "books", "not_found");
/// A code in the `crumbkit` component that no list holds.
const UNLISTED: Code = Code::new("crumbkit", "books", "unlisted");
/// The tool's code that a guide prints in place of `UNLISTED`.
const RENAMED: Code = Code::new("tool", "books", "renamed");

/// The script's answers: `books show 7` fails with `NOT_FOUND`, `books show
/// 5` fails with crumbkit-child's code `tool::child::not_found`, `books
/// show 6` fails with `UNLISTED`, which prints with the tool's name, and
/// `books show 2` exits with code 0, with or without `--json` at the end.
/// Any other command line fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'books show 7') printf 'error: no book 7\n  code:  tool::books::not_found\n' >&2; exit 1;;
  'books show 5') printf 'error: cannot run `viewer`: no such program\n  code:  tool::child::not_found\n' >&2; exit 1;;
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

/// The lists of a tool that also uses crumbkit-child.
const WITH_CHILD: &[&[&Code]] = &[crumbkit::codes::ALL, crumbkit_child::codes::ALL];

/// The settings of one run of `check_twins`.
struct Run<'a> {
    /// The tool's declared codes.
    codes: &'a [&'a [&'a Code]],
    /// The twins.
    pairs: &'a [Twin<'a>],
    /// The opt-outs.
    allow: &'a [Allow],
    /// The tool's guide.
    guide: Guide,
    /// The lists of library codes.
    trusted: &'a [&'a [&'a Code]],
}

impl<'a> Run<'a> {
    /// Returns a run of the tool with `codes` and `pairs`, no opt-out, a
    /// guide that declares nothing and the lists of `TRUSTED`.
    fn new(codes: &'a [&'a [&'a Code]], pairs: &'a [Twin<'a>]) -> Self {
        Self {
            codes,
            pairs,
            allow: &[],
            guide: Guide::new(),
            trusted: TRUSTED,
        }
    }

    /// Returns the panic message of `check_twins`, or `None` when it passes.
    fn checked(&self) -> Option<String> {
        let dir = TempDir::new("twins-trusted").unwrap();
        dir.write_script("tool", SCRIPT);
        let path = dir.path().join("tool").display().to_string();
        let twins =
            Twins::new(Examples::new(&path, "").allow(self.allow), self.codes).pairs(self.pairs);
        panic_message(|| check_twins(&tree(), &self.guide, &twins, self.trusted))
    }
}

#[test]
fn a_twin_of_a_code_that_a_trusted_list_holds_needs_no_declaration() {
    let run = Run {
        trusted: WITH_CHILD,
        ..Run::new(&[&[&NOT_FOUND]], &[SHOW_7, SHOW_5])
    };

    assert_eq!(run.checked(), None);
}

#[test]
fn a_declared_code_that_a_trusted_list_holds_needs_no_twin() {
    let run = Run {
        trusted: WITH_CHILD,
        ..Run::new(
            &[&[&NOT_FOUND, &crumbkit_child::codes::NOT_STARTED]],
            &[SHOW_7],
        )
    };

    assert_eq!(run.checked(), None);
}

#[test]
fn a_twin_that_refuses_a_crumbkit_component_code_that_no_list_holds_fails_as_not_declared() {
    let run = Run::new(&[&[&NOT_FOUND]], &[SHOW_7, SHOW_6]);

    assert_eq!(
        run.checked().as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the refused line `tool books show 6` fails with tool::books::unlisted, which is not a declared code or a code that a trusted list holds"
        )
    );
}

#[test]
fn a_twin_of_a_child_code_fails_as_not_declared_when_no_list_holds_the_child_codes() {
    let run = Run::new(&[&[&NOT_FOUND]], &[SHOW_7, SHOW_5]);

    assert_eq!(
        run.checked().as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             the refused line `tool books show 5` fails with tool::child::not_found, which is not a declared code or a code that a trusted list holds"
        )
    );
}

#[test]
fn a_declared_crumbkit_component_code_that_no_list_holds_needs_a_twin() {
    let run = Run::new(&[&[&NOT_FOUND, &UNLISTED]], &[SHOW_7]);

    assert_eq!(
        run.checked().as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::unlisted: no twin refuses the declared code, Twins::unprovoked does not list it, and no opt-out made with Allow::untwinned names it"
        )
    );
}

#[test]
fn a_declared_tool_code_renamed_from_a_crumbkit_component_code_that_no_list_holds_needs_a_twin() {
    const GUIDE: Guide = Guide::new().codes(&[Rename::new(&UNLISTED, &RENAMED)]);
    let run = Run {
        guide: GUIDE,
        ..Run::new(&[&[&NOT_FOUND, &RENAMED]], &[SHOW_7])
    };

    assert_eq!(
        run.checked().as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::renamed: no twin refuses the declared code, Twins::unprovoked does not list it, and no opt-out made with Allow::untwinned names it"
        )
    );
}

#[test]
fn an_exit_that_lists_a_crumbkit_component_code_that_no_list_holds_fails_the_twin_check() {
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(3, "Unlisted.").failures(&[&UNLISTED])]);
    let run = Run {
        guide: GUIDE,
        ..Run::new(&[&[&NOT_FOUND]], &[SHOW_7])
    };

    assert_eq!(
        run.checked().as_deref(),
        Some(
            "the tool's twins break crumbkit's twin rule:\n\
             tool::books::unlisted: the guide's exit code 3 lists the failure code, and no CODES list holds it"
        )
    );
}

#[test]
fn an_untwinned_opt_out_for_a_declared_crumbkit_component_code_that_no_list_holds_is_accepted() {
    const ALLOW: &[Allow] = &[Allow::untwinned(
        Failing::new(&UNLISTED, &["books", "show", "6"]),
        "A broken file causes it, and no single word does.",
    )];
    let run = Run {
        allow: ALLOW,
        ..Run::new(&[&[&NOT_FOUND, &UNLISTED]], &[SHOW_7])
    };

    assert_eq!(run.checked(), None);
}
