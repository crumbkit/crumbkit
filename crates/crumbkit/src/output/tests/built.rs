//! Tests for an `Output` that a tool builds with its builder, in place of
//! flattening it into its clap parser.

use clap::CommandFactory as _;

use super::{PLANETS, Tool, finish_output, finish_with, mercury, saturn};
use crate::Output;

/// Returns `output` with the command of the test tool set, as a tool's
/// `main` sets it after building the flags itself.
fn with_command(mut output: Output) -> Output {
    output.set_command(Tool::command());
    output
}

#[test]
fn an_output_built_with_json_and_fields_prints_what_the_same_parsed_flags_print() {
    let mut built = with_command(Output::new().json(true).fields(vec!["name".to_owned()]));

    let finished = finish_output(&mut built, PLANETS, vec![mercury(), saturn()]);
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
    let mut built = with_command(Output::new().long(true));

    let finished = finish_output(&mut built, PLANETS, vec![mercury()]);
    let parsed = finish_with(&["--long"], PLANETS, vec![mercury()]);

    assert_eq!(finished.out, parsed.out);
    assert_eq!(
        finished.out,
        "NAME     MOONS  RINGED\nMercury  0      false\n"
    );
}

#[test]
fn an_output_built_with_full_and_without_json_is_refused_as_the_parsed_flags_are() {
    let mut built = with_command(Output::new().full(true));
    let mut admitted = with_command(Output::new().full(true).json(true));

    let finished = finish_output(&mut built, PLANETS, vec![mercury()]);
    let twin = finish_output(&mut admitted, PLANETS, vec![mercury()]);

    assert_eq!(finished.code, std::process::ExitCode::from(2));
    assert_eq!(finished.out, "");
    assert!(
        finished
            .err
            .contains("--full prints every field of every row in JSON")
    );
    assert_eq!(twin.code, std::process::ExitCode::SUCCESS);
}
