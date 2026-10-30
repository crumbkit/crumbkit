//! Tests for an `Output` that a tool builds with `Output::new`, from view
//! flags that clap parsed or that the tool built itself.

use clap::CommandFactory as _;

use super::{PLANETS, Tool, build_output, finish_output, finish_with, mercury, saturn};
use crate::help::Guide;
use crate::{Output, ViewFlags};

/// Returns the output of the test tool's `tool list` with `view`, as a
/// tool's `main` builds it after it parsed with clap alone or built the
/// flags itself.
fn built(view: ViewFlags) -> Output {
    Output::new(
        view,
        crate::DryRun::default(),
        Tool::command(),
        &Guide::new(),
        ["tool", "list"],
    )
    .unwrap()
}

#[test]
fn an_output_built_with_json_and_fields_prints_what_the_same_parsed_flags_print() {
    let built = built(ViewFlags::new().json(true).fields(vec!["name".to_owned()]));

    let finished = finish_output(built, PLANETS, vec![mercury(), saturn()]);
    let parsed = finish_with(
        &["--json", "--fields", "name"],
        PLANETS,
        vec![mercury(), saturn()],
    );

    assert_eq!(finished.code, parsed.code);
    assert_eq!(finished.out, parsed.out);
    assert_eq!(finished.err, parsed.err);
}

#[test]
fn an_output_built_with_long_prints_the_long_columns_as_the_parsed_flag_does() {
    let built = built(ViewFlags::new().long(true));

    let finished = finish_output(built, PLANETS, vec![mercury()]);
    let parsed = finish_with(&["--long"], PLANETS, vec![mercury()]);

    assert_eq!(finished.out, parsed.out);
    assert_eq!(
        finished.out,
        "NAME     MOONS  RINGED\nMercury  0      false\n"
    );
}

#[test]
fn an_output_built_with_full_and_without_json_is_refused_as_the_parsed_flags_are() {
    let refused = build_output(ViewFlags::new().full(true), &["tool", "list"]);
    let admitted = build_output(ViewFlags::new().full(true).json(true), &["tool", "list"]);

    let stop = refused.result.unwrap_err();
    assert_eq!(stop.exit(), 2);
    assert_eq!(refused.out, "");
    assert!(
        refused
            .err
            .contains("--full prints every field of every row in JSON")
    );
    let admitted = admitted.result.unwrap();
    let twin = finish_output(admitted, PLANETS, vec![mercury()]);
    assert_eq!(twin.code, std::process::ExitCode::SUCCESS);
}

#[test]
fn output_new_reports_its_refusal_of_full_without_json_as_a_printed_refusal() {
    let refused = build_output(ViewFlags::new().full(true), &["tool", "list"]);
    let admitted = build_output(ViewFlags::new().full(true).json(true), &["tool", "list"]);

    let printed = refused.result.as_ref().err().map(crate::Stop::printed);

    assert_eq!(printed, Some(crate::Printed::Refusal));
    assert!(admitted.result.is_ok(), "{}", admitted.err);
}

#[test]
fn output_new_keeps_the_words_after_the_program_name() {
    let output = Output::new(
        ViewFlags::new(),
        crate::DryRun::default(),
        Tool::command(),
        &Guide::new(),
        ["tool", "show", "Mars"],
    )
    .unwrap();

    assert_eq!(
        output.words(),
        ["show", "Mars"].map(std::ffi::OsString::from)
    );
}

#[test]
fn output_new_holds_the_command_that_it_was_given() {
    let mut output = built(ViewFlags::new());

    assert_eq!(output.command().get_name(), "tool");
    assert!(output.command().find_subcommand("show").is_some());
}
