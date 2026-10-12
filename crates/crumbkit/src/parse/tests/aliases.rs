//! Tests for the line that names a person's aliases on the root's help.

use std::process::ExitCode;

use clap::Command;

use super::tool;
use crate::help::Guide;
use crate::parse::parse_to;
use crate::{Sink, Spelling};

/// Returns two alias names, the second built from the name of `tool`, so a
/// test sees which command the function received.
fn names(tool: &Command) -> Vec<String> {
    vec!["firsts".to_owned(), format!("{}-mine", tool.get_name())]
}

/// Fails the test, for a guide whose aliases a run must not read.
fn unread(tool: &Command) -> Vec<String> {
    panic!("the run read the personal aliases of {}", tool.get_name());
}

/// Returns no alias names, as for a person without an aliases file.
fn none(_: &Command) -> Vec<String> {
    Vec::new()
}

/// The line that the root's help prints with [`names`].
const LINE: &str =
    "Personal aliases: firsts, tool-mine. `tool aliases list` shows what each runs.\n";

/// Runs `parse_to` over `tool()` with `guide` and `args`, and returns the
/// exit code and what it printed to the output writer.
fn run(guide: &Guide, args: &[&str]) -> (Option<ExitCode>, String) {
    let mut out = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, Vec::new()),
        tool(),
        guide,
        Spelling::COLONS,
        args,
    );
    (
        result.err().map(ExitCode::from),
        String::from_utf8(out).unwrap(),
    )
}

#[test]
fn the_root_s_help_names_the_personal_aliases_between_the_topics_line_and_the_next_line() {
    const GUIDE: Guide = Guide::new().personal_aliases(names);

    for flag in ["-h", "--help"] {
        let (code, page) = run(&GUIDE, &["tool", flag]);

        assert_eq!(code, Some(ExitCode::SUCCESS));
        let line = page.find(LINE).unwrap_or_else(|| panic!("{page}"));
        assert!(page.find("Topics: ").unwrap() < line, "{page}");
        assert!(line < page.find("Next: ").unwrap(), "{page}");
    }
}

#[test]
fn a_run_that_prints_no_root_help_never_reads_the_personal_aliases() {
    const GUIDE: Guide = Guide::new().personal_aliases(unread);

    for args in [
        ["tool", "planets", "show", "mars"].as_slice(),
        &["tool", "planets", "-h"],
        &["tool", "--version"],
        &["tool"],
        &["tool", "help"],
    ] {
        let _ = run(&GUIDE, args);
    }
}

#[test]
fn a_person_without_aliases_gets_the_root_s_help_without_the_line() {
    const GUIDE: Guide = Guide::new().personal_aliases(none);

    assert_eq!(
        run(&GUIDE, &["tool", "-h"]),
        run(&Guide::new(), &["tool", "-h"])
    );
}

#[test]
fn a_guide_without_personal_aliases_prints_the_root_s_help_without_the_line() {
    const GUIDE: Guide = Guide::new()
        .personal_aliases(names)
        .without_personal_aliases();

    assert!(GUIDE.get_personal_aliases().is_none());
    assert_eq!(
        run(&GUIDE, &["tool", "-h"]),
        run(&Guide::new(), &["tool", "-h"])
    );
}
