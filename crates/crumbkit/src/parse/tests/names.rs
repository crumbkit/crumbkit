//! Tests that a refusal names the tool, as its root command or its tool
//! record's root names it, in the failure document's id and in crumbkit's
//! codes.

use crate::parse::tests::parse_tree_to;
use std::process::ExitCode;

use clap::{Arg, ArgAction, Command};

use super::{Parsed, tree};
use crate::help::Help;
use crate::parse::Stop;
use crate::{Sink, Tool};

/// Runs `parse_to` over `command` with `tool` and `args` into memory, for
/// a command line that does not parse.
fn parse(command: Command, tool: &Tool, args: &[&str]) -> Parsed {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let result = parse_tree_to::<()>(&mut Sink::new(&mut out, &mut err), command, tool, args);
    let stop = result.err();
    Parsed {
        code: stop.as_ref().map(Stop::exit_code),
        json: stop.as_ref().map(|stop| stop.run().is_json()),
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
            Arg::new(crate::ViewFlags::JSON_ID)
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue),
        )
        .subcommand(Command::new("run").about("Run the tool"))
}

#[test]
fn a_refusal_with_json_prints_the_tools_failure_document_and_the_tools_usage_code() {
    let parsed = parse(tree(), &Tool::new(), &["tool", "planets", "show", "--json"]);

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
        &Tool::new(),
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
fn a_tool_whose_record_roots_it_below_another_program_is_named_by_that_root() {
    const TOOL: Tool = Tool::new().help(Help::new().root(&["planets"]));
    let parsed = parse(tree(), &TOOL, &["tool", "planets", "show"]);

    assert!(
        parsed.err.ends_with("  code:  planets::cli::usage\n"),
        "{}",
        parsed.err
    );
}
