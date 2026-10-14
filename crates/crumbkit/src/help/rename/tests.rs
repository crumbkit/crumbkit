//! Tests for the codes that a guide prints in place of crumbkit's, and the
//! exit code that a failure ends a run with.

use crate::Code;
use crate::codes::{CLI_USAGE, OUTPUT_OVER_BUDGET, VERSION_CONFLICT};
use crate::help::{Exit, Guide, Leaf, Rename};

const USAGE: Code = Code::new("ledger", "cli", "usage");
const STALE: Code = Code::new("ledger", "rows", "stale");
const LOCKED: Code = Code::new("ledger", "rows", "locked");
const CHANGED: Code = Code::new("ledger", "rows", "changed");
const TOO_LONG: Code = Code::new("ledger", "rows", "too_long");

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
    assert_eq!(GUIDE.printed(&[], &CLI_USAGE), &USAGE);
    assert_eq!(GUIDE.printed(&[], &VERSION_CONFLICT), &STALE);
}

#[test]
fn a_guide_prints_a_code_that_it_does_not_rename_unchanged() {
    assert_eq!(GUIDE.printed(&[], &OUTPUT_OVER_BUDGET), &OUTPUT_OVER_BUDGET);
    assert_eq!(GUIDE.printed(&[], &LOCKED), &LOCKED);
    assert_eq!(Guide::new().printed(&[], &CLI_USAGE), &CLI_USAGE);
}

#[test]
fn a_failure_exits_with_the_number_that_the_guide_lists_for_its_printed_code() {
    assert_eq!(GUIDE.failure_exit(&[], &CLI_USAGE), 64);
    assert_eq!(GUIDE.failure_exit(&[], &USAGE), 64);
    assert_eq!(GUIDE.failure_exit(&[], &LOCKED), 3);
}

#[test]
fn an_exit_that_lists_a_crumbkit_code_also_covers_the_tool_s_rename_of_it() {
    const LISTS_CRUMBKIT: Guide = Guide::new()
        .codes(&[Rename::new(&VERSION_CONFLICT, &STALE)])
        .exits(
            &[Exit::new(5, "The row changed since it was read.").failures(&[&VERSION_CONFLICT])],
        );
    assert_eq!(LISTS_CRUMBKIT.failure_exit(&[], &VERSION_CONFLICT), 5);
    assert_eq!(LISTS_CRUMBKIT.failure_exit(&[], &STALE), 5);
}

#[test]
fn a_failure_that_no_exit_lists_exits_with_1_and_a_refusal_with_2() {
    assert_eq!(GUIDE.failure_exit(&[], &VERSION_CONFLICT), 1);
    assert_eq!(Guide::new().failure_exit(&[], &LOCKED), 1);
    assert_eq!(Guide::new().failure_exit(&[], &CLI_USAGE), 2);
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

/// A guide with `GUIDE`'s renames and exits, whose `rows edit` leaf renames
/// the conflict code and the over-budget code, and whose exit 6 lists the
/// leaf's conflict code.
const LEAF_GUIDE: Guide = Guide::new()
    .codes(&[
        Rename::new(&CLI_USAGE, &USAGE),
        Rename::new(&VERSION_CONFLICT, &STALE),
    ])
    .exits(&[
        Exit::new(64, "The command line was refused.").failures(&[&USAGE]),
        Exit::new(6, "The row changed while it was edited.").failures(&[&CHANGED]),
    ])
    .leaves(&[Leaf::new(&["rows", "edit"]).codes(&[
        Rename::new(&VERSION_CONFLICT, &CHANGED),
        Rename::new(&OUTPUT_OVER_BUDGET, &TOO_LONG),
    ])]);

#[test]
fn a_leaf_s_rename_prints_in_place_of_the_guide_s_for_runs_of_that_command_only() {
    assert_eq!(
        LEAF_GUIDE.printed(&["rows", "edit"], &VERSION_CONFLICT),
        &CHANGED
    );
    assert_eq!(
        LEAF_GUIDE.printed(&["rows", "list"], &VERSION_CONFLICT),
        &STALE
    );
    assert_eq!(LEAF_GUIDE.printed(&[], &VERSION_CONFLICT), &STALE);
}

#[test]
fn a_leaf_s_rename_of_a_code_that_the_guide_does_not_rename_prints_for_that_command_only() {
    assert_eq!(
        LEAF_GUIDE.printed(&["rows", "edit"], &OUTPUT_OVER_BUDGET),
        &TOO_LONG
    );
    assert_eq!(
        LEAF_GUIDE.printed(&["rows", "list"], &OUTPUT_OVER_BUDGET),
        &OUTPUT_OVER_BUDGET
    );
}

#[test]
fn a_leaf_keeps_the_guide_s_renames_of_the_codes_that_it_does_not_rename() {
    assert_eq!(LEAF_GUIDE.printed(&["rows", "edit"], &CLI_USAGE), &USAGE);
    assert_eq!(LEAF_GUIDE.failure_exit(&["rows", "edit"], &CLI_USAGE), 64);
}

#[test]
fn an_exit_that_lists_a_leaf_s_code_covers_the_failure_of_that_command_only() {
    assert_eq!(
        LEAF_GUIDE.failure_exit(&["rows", "edit"], &VERSION_CONFLICT),
        6
    );
    assert_eq!(LEAF_GUIDE.failure_exit(&["rows", "edit"], &CHANGED), 6);
    assert_eq!(
        LEAF_GUIDE.failure_exit(&["rows", "list"], &VERSION_CONFLICT),
        1
    );
}
