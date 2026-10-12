//! Tests for `--expect` and the conflict that a stale version fails with.

use std::process::ExitCode;

use clap::{CommandFactory as _, Parser};

use super::{Expect, Expected};
use crate::help::Guide;
use crate::twins::assert_one_word_apart;
use crate::{Call, Error, Field, Recovery, Sink, Spelling};

/// A command that sets a planet's name and takes `--expect`.
#[derive(Parser)]
#[command(name = "rename")]
struct Rename {
    name: String,
    #[command(flatten)]
    expect: Expect,
}

/// Parses `rename` followed by `words` with crumbkit's `parse_to`, and
/// returns the flag, or the exit code of a refusal.
fn expect(words: &[&str]) -> Result<Expect, ExitCode> {
    let args = std::iter::once("rename").chain(words.iter().copied());
    let mut sink = Sink::new(Vec::new(), Vec::new());
    crate::parse_to::<Rename, _, _, _, _>(
        &mut sink,
        Rename::command(),
        &Guide::new(),
        Spelling::COLONS,
        args,
    )
    .map(|(rename, _)| rename.expect)
    .map_err(ExitCode::from)
}

/// The command that prints a planet's current name, which a conflict's
/// recovery runs.
fn read() -> Call {
    Call::new(&["planets", "show"]).value("name", "ceres")
}

/// Returns the code of a failed result in the colon spelling.
fn code(result: &Result<(), Error>) -> String {
    let error = result.as_ref().unwrap_err();
    Spelling::COLONS.spell(error.failure().code())
}

#[test]
fn an_expect_that_is_left_out_reads_as_no_precondition() {
    let flag = expect(&["pluto"]).unwrap();

    assert_eq!(flag.get(), None);
    assert!(flag.check(Some("7"), read()).is_ok());
    assert!(flag.check(None, read()).is_ok());
}

#[test]
fn expect_none_reads_as_a_target_that_must_not_exist() {
    let flag = expect(&["pluto", "--expect", "none"]).unwrap();

    assert_eq!(flag.get(), Some(Expected::Nothing));
    assert!(flag.check(None, read()).is_ok());
}

#[test]
fn an_empty_expect_is_refused_and_its_twin_with_a_version_is_admitted() {
    let refused = ["pluto", "--expect", ""];
    let admitted = ["pluto", "--expect", "7"];
    assert_one_word_apart(&refused, &admitted);

    assert_eq!(
        expect(&admitted).unwrap().get(),
        Some(Expected::Version("7"))
    );
    assert_eq!(expect(&refused).err(), Some(ExitCode::from(2)));
}

#[test]
fn check_passes_when_the_current_version_matches_the_expected_one() {
    let flag = expect(&["pluto", "--expect", "7"]).unwrap();

    assert!(flag.check(Some("7"), read()).is_ok());
}

#[test]
fn check_fails_with_a_version_conflict_when_the_versions_differ() {
    let flag = expect(&["pluto", "--expect", "7"]).unwrap();

    let result = flag.check(Some("8"), read());

    assert_eq!(code(&result), "crumbkit::version::conflict");
    assert_eq!(
        result.unwrap_err().to_string(),
        "the target is at version 8, and --expect named version 7, so the command changed nothing"
    );
}

#[test]
fn check_fails_with_a_conflict_when_expect_none_meets_an_existing_target() {
    let flag = expect(&["pluto", "--expect", "none"]).unwrap();

    let result = flag.check(Some("8"), read());

    assert_eq!(code(&result), "crumbkit::version::conflict");
    assert_eq!(
        result.unwrap_err().to_string(),
        "the target exists at version 8, and --expect none requires that it does not exist, so the command changed nothing"
    );
}

#[test]
fn check_fails_with_a_conflict_when_a_version_is_expected_and_no_target_exists() {
    let flag = expect(&["pluto", "--expect", "7"]).unwrap();

    let result = flag.check(None, read());

    assert_eq!(code(&result), "crumbkit::version::conflict");
    assert_eq!(
        result.unwrap_err().to_string(),
        "the target does not exist, and --expect named version 7, so the command changed nothing"
    );
}

#[test]
fn the_conflict_runs_the_read_command_the_tool_names_and_its_fields_name_both_versions() {
    let flag = expect(&["pluto", "--expect", "none"]).unwrap();

    let error = flag.conflict(Some("8"), read());
    let failure = error.failure();

    assert_eq!(failure.recovery(), Recovery::Run(read()));
    assert_eq!(
        failure.fields(),
        [Field::new("expected", "none"), Field::new("current", "8")]
    );
}
