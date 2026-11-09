//! Tests for the convention suite and the macro that expands it.

use clap::Command;
use crumbkit::{Code, Tool};

use super::{Check, Suite};
use crate::testing::panic_message;
use crate::{Allow, Examples, Rule};

const TOOL: Tool = Tool::new();

/// Returns a tree whose one problem is that help shows a second name for
/// `books`.
fn second_name() -> Command {
    Command::new("shelf").subcommand(
        Command::new("books")
            .visible_alias("b")
            .subcommand(Command::new("list")),
    )
}

/// Returns the tree of [`second_name`] with the tool record that a tool builds
/// at run time.
fn built() -> (Command, Tool) {
    (second_name(), TOOL)
}

const SUITE: Suite<'_> = Suite::new(second_name, &TOOL);

#[test]
fn a_check_whose_input_is_missing_fails_and_names_the_input() {
    for (check, input) in [
        (Check::PathWeights, ".path_budgets("),
        (Check::Codes, ".codes("),
        (Check::Examples, ".examples("),
        (Check::OutputBudget, ".examples("),
        (Check::CommandReference, ".examples("),
        (Check::Recoveries, ".recoveries("),
        (Check::Twins, ".twins("),
    ] {
        let message = panic_message(|| SUITE.run(check)).unwrap_or_default();
        assert!(message.contains(input), "{check:?}: {message}");
        assert!(
            message.contains(&format!(".skip(Check::{check:?},")),
            "{check:?}: {message}"
        );
    }
}

#[test]
fn a_skip_with_a_reason_passes_a_check_whose_input_is_missing() {
    const SKIPS: Suite<'_> = SUITE.skip(Check::Codes, "shelf declares no codes.");

    assert_eq!(panic_message(|| SKIPS.run(Check::Codes)), None);
}

#[test]
fn a_skip_leaves_every_other_check_running() {
    const SKIPS: Suite<'_> = SUITE.skip(Check::Codes, "shelf declares no codes.");

    let message = panic_message(|| SKIPS.run(Check::Twins)).unwrap_or_default();
    assert!(message.contains(".twins("), "{message}");
}

#[test]
fn a_check_runs_on_the_tools_command_tree_and_reports_its_problems() {
    let message = panic_message(|| SUITE.run(Check::Names)).unwrap_or_default();
    assert!(message.contains("books"), "{message}");
}

#[test]
fn a_tool_built_at_run_time_is_checked_as_a_declared_tool_is() {
    let message = panic_message(|| Suite::built(built).run(Check::Names)).unwrap_or_default();
    assert!(message.contains("books"), "{message}");
}

#[test]
fn the_opt_outs_reach_the_checks() {
    const ALLOWS: Suite<'_> = SUITE.allow(&[Allow::new(
        Rule::OneName,
        "books",
        "People type `shelf b` from habit.",
    )]);

    assert_eq!(panic_message(|| ALLOWS.run(Check::Names)), None);
}

#[test]
fn the_codes_reach_the_codes_check() {
    const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
    const LIST: &[&Code] = &[&NOT_FOUND];
    const CODES: Suite<'_> = SUITE.codes(&[LIST]);

    assert_eq!(panic_message(|| CODES.run(Check::Codes)), None);
}

#[test]
fn the_golden_directory_holds_the_help_golden_files() {
    const GOLDENS: Suite<'_> = SUITE.goldens("target/no-such-goldens");

    let message = panic_message(|| GOLDENS.run(Check::HelpGoldens)).unwrap_or_default();
    assert!(message.contains("no-such-goldens/help"), "{message}");
}

#[test]
fn binary_help_pages_need_the_examples_input() {
    const BINARY: Suite<'_> = SUITE.help_from_binary();

    let message = panic_message(|| BINARY.run(Check::HelpSizes)).unwrap_or_default();
    assert!(message.contains(".examples("), "{message}");
}

#[cfg(unix)]
#[test]
fn binary_help_pages_are_what_the_binary_prints() {
    let dir = crate::temp::TempDir::new("suite-binary").unwrap();
    let binary = crate::testing::help_binary(&dir, &[("-h", "shelf\n")]);
    let suite = SUITE
        .examples(Examples::new(&binary, "tests/golden/examples"))
        .help_from_binary();

    let message = panic_message(|| suite.run(Check::HelpSizes)).unwrap_or_default();
    assert!(message.contains("books -h"), "{message}");
}

/// Returns `suite` with every check skipped.
const fn skip_every_check(mut suite: Suite<'static>) -> Suite<'static> {
    let mut index = 0;
    while index < Check::ALL.len() {
        suite = suite.skip(Check::ALL[index], "This suite tests the macro.");
        index += 1;
    }
    suite
}

/// The tests that the macro expands to, which pass because every check is
/// skipped.
mod expanded {
    use super::{SUITE, Suite, skip_every_check};

    const EVERY_CHECK_SKIPPED: Suite<'_> = skip_every_check(SUITE);

    conventions!(EVERY_CHECK_SKIPPED);

    #[test]
    fn the_macro_expands_to_one_named_test_for_each_check() {
        every_command_and_argument_has_one_name();
        every_declared_code_has_a_twin_that_the_tool_accepts();
    }
}
