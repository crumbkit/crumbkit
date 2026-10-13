//! Tests for the codes that a guide prints in place of crumbkit's, and the
//! exit code that a failure ends a run with.

use crate::Code;
use crate::codes::{CLI_USAGE, OUTPUT_OVER_BUDGET, VERSION_CONFLICT};
use crate::help::{Exit, Guide, Rename};

const USAGE: Code = Code::new("ledger", "cli", "usage");
const STALE: Code = Code::new("ledger", "rows", "stale");
const LOCKED: Code = Code::new("ledger", "rows", "locked");

/// A guide that renames the usage and conflict codes, and lists the usage
/// code under 64 and the tool's own `LOCKED` under 3.
const GUIDE: Guide = Guide::new()
    .codes(&[
        Rename::new(&CLI_USAGE, &USAGE),
        Rename::new(&VERSION_CONFLICT, &STALE),
    ])
    .exits(&[
        Exit::new(64, "The command line was refused.").failures(&[&USAGE]),
        Exit::new(3, "The ledger was locked.").failures(&[&LOCKED]),
    ]);

#[test]
fn a_guide_prints_a_renamed_crumbkit_code_as_the_tool_s_code() {
    assert_eq!(GUIDE.printed(&CLI_USAGE), &USAGE);
    assert_eq!(GUIDE.printed(&VERSION_CONFLICT), &STALE);
}

#[test]
fn a_guide_prints_a_code_that_it_does_not_rename_unchanged() {
    assert_eq!(GUIDE.printed(&OUTPUT_OVER_BUDGET), &OUTPUT_OVER_BUDGET);
    assert_eq!(GUIDE.printed(&LOCKED), &LOCKED);
    assert_eq!(Guide::new().printed(&CLI_USAGE), &CLI_USAGE);
}

#[test]
fn a_failure_exits_with_the_number_that_the_guide_lists_for_its_printed_code() {
    assert_eq!(GUIDE.failure_exit(&CLI_USAGE), 64);
    assert_eq!(GUIDE.failure_exit(&USAGE), 64);
    assert_eq!(GUIDE.failure_exit(&LOCKED), 3);
}

#[test]
fn an_exit_that_lists_a_crumbkit_code_also_covers_the_tool_s_rename_of_it() {
    const LISTS_CRUMBKIT: Guide = Guide::new()
        .codes(&[Rename::new(&VERSION_CONFLICT, &STALE)])
        .exits(
            &[Exit::new(5, "The row changed since it was read.").failures(&[&VERSION_CONFLICT])],
        );
    assert_eq!(LISTS_CRUMBKIT.failure_exit(&VERSION_CONFLICT), 5);
    assert_eq!(LISTS_CRUMBKIT.failure_exit(&STALE), 5);
}

#[test]
fn a_failure_that_no_exit_lists_exits_with_1_and_a_refusal_with_2() {
    assert_eq!(GUIDE.failure_exit(&VERSION_CONFLICT), 1);
    assert_eq!(Guide::new().failure_exit(&LOCKED), 1);
    assert_eq!(Guide::new().failure_exit(&CLI_USAGE), 2);
}

#[cfg(feature = "hand-over")]
#[test]
fn a_child_that_cannot_answer_json_exits_as_a_refused_command_line_does() {
    use crate::codes::CHILD_NO_JSON;
    assert_eq!(Guide::new().failure_exit(&CHILD_NO_JSON), 2);
    assert_eq!(GUIDE.failure_exit(&CHILD_NO_JSON), 64);
}

#[test]
fn a_refused_command_line_prints_the_tool_s_usage_code_and_exits_with_the_code_its_guide_lists() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let command = clap::Command::new("ledger").arg(clap::Arg::new("id"));

    let stop = crate::parse_to::<(), _, _, _, _>(
        &mut crate::Sink::new(&mut out, &mut err),
        command,
        &GUIDE,
        crate::Spelling::COLONS,
        ["ledger", "--srot"],
    )
    .err()
    .unwrap();

    assert_eq!(stop.code(), std::process::ExitCode::from(64));
    let err = String::from_utf8(err).unwrap();
    assert!(err.ends_with("  code:  ledger::cli::usage\n"), "{err}");
}
