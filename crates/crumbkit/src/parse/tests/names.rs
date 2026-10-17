//! Tests that a refusal names the tool, as its root command or its guide's
//! root names it, in the failure document's id and in crumbkit's codes.

use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::{Parsed, tool};
use crate::help::{Guide, Rename};
use crate::parse::{Stop, parse_to};
use crate::{Code, Sink, Spelling, codes};

/// Runs `parse_to` over `command` with `guide` and `args` into memory, for
/// a command line that does not parse.
fn parse(command: Command, guide: &Guide, args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_to::<(), _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        command,
        guide,
        Spelling::COLONS,
        args,
    );
    let stop = result.err();
    Parsed {
        code: stop.as_ref().map(Stop::code),
        json: stop.as_ref().map(|stop| stop.output().is_json()),
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// A tool named `my-tool` with a `--json` flag and one leaf that takes no
/// arguments.
fn hyphenated() -> Command {
    Command::new("my-tool")
        .subcommand_required(true)
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(Command::new("run").about("Run the tool"))
}

#[test]
fn a_refusal_with_json_prints_the_tools_failure_document_and_the_tools_usage_code() {
    let parsed = parse(
        tool(),
        &Guide::new(),
        &["tool", "planets", "show", "--json"],
    );

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(
        parsed
            .out
            .starts_with(r#"{"schema":"tool/failure/v1","code":"tool::cli::usage","#),
        "{}",
        parsed.out
    );
}

#[test]
fn a_tool_whose_name_has_a_hyphen_keeps_it_in_the_schema_id_and_spells_its_code_with_an_underscore()
{
    let parsed = parse(
        hyphenated(),
        &Guide::new(),
        &["my-tool", "run", "--srot", "--json"],
    );

    assert_eq!(parsed.code, Some(ExitCode::from(2)));
    assert!(
        parsed
            .out
            .starts_with(r#"{"schema":"my-tool/failure/v1","code":"my_tool::cli::usage","#),
        "{}",
        parsed.out
    );
}

#[test]
fn a_tool_that_renames_the_usage_code_prints_its_own_code_in_place_of_the_tools_name() {
    const REFUSED: Code = Code::new("tool", "command_line", "refused");
    const GUIDE: Guide = Guide::new().codes(&[Rename::new(&codes::CLI_USAGE, &REFUSED)]);
    let parsed = parse(tool(), &GUIDE, &["tool", "planets", "show"]);

    assert!(
        parsed
            .err
            .ends_with("  code:  tool::command_line::refused\n"),
        "{}",
        parsed.err
    );
}

#[test]
fn a_tool_whose_guide_roots_it_below_another_program_is_named_by_that_root() {
    const GUIDE: Guide = Guide::new().root(&["planets"]);
    let parsed = parse(tool(), &GUIDE, &["tool", "planets", "show"]);

    assert!(
        parsed.err.ends_with("  code:  planets::cli::usage\n"),
        "{}",
        parsed.err
    );
}
