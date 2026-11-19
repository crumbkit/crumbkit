//! Tests for the tool record check of guide topics.

use clap::Command;
use crumbkit::Tool;
use crumbkit::help::{Help, Step, Topic};

use super::{check_help, panic_message, shelf, shelf_with_arguments};

/// The help rules' heading in a panic message.
const HEAD: &str = "the command tree breaks crumbkit's help rules:\n";

/// Runs `check_help` over `tree` with `tool` and returns the panic message,
/// or `None` when the tool record passes.
fn problems(tree: &Command, tool: &Tool) -> Option<String> {
    panic_message(|| check_help(tree, tool, &[]))
}

/// A guide of two steps whose commands parse against `shelf_with_arguments`.
const SHOW: Topic = Topic::guide(
    "show-books",
    "List and show books",
    &[
        Step::new("List every book.", &[&["books", "list", "--all"]]),
        Step::new("Show two of them.", &[&["books", "show", "2", "5"]]),
    ],
);

#[test]
fn a_guide_whose_commands_parse_passes() {
    const TOOL: Tool = Tool::new().help(Help::new().topics(&[SHOW]));
    assert_eq!(problems(&shelf_with_arguments(), &TOOL), None);
}

#[test]
fn check_help_fails_a_guide_command_that_clap_refuses() {
    const BAD: Topic = Topic::guide(
        "show-books",
        "",
        &[Step::new(
            "List every book.",
            &[&["books", "list", "--everything"]],
        )],
    );
    const TOOL: Tool = Tool::new().help(Help::new().topics(&[BAD]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `show-books` step 1: clap refuses the command `shelf books list --everything` with the error kind UnknownArgument"
        )
    );
}

#[test]
fn check_help_names_the_step_number_and_quotes_the_command_words() {
    const BAD: Topic = Topic::guide(
        "show-books",
        "",
        &[
            Step::new("List.", &[&["books", "list"]]),
            Step::new("Show.", &[&["books", "show", "1"], &["books", "show"]]),
        ],
    );
    const TOOL: Tool = Tool::new().help(Help::new().topics(&[BAD]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `show-books` step 2: clap refuses the command `shelf books show` with the error kind MissingRequiredArgument"
        )
    );
}

#[test]
fn check_help_counts_help_and_version_in_a_guide_as_refused() {
    const BAD: Topic = Topic::guide(
        "show-books",
        "",
        &[Step::new("Ask.", &[&["--version"], &["books", "--help"]])],
    );
    const TOOL: Tool = Tool::new().help(Help::new().topics(&[BAD]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `show-books` step 1: clap refuses the command `shelf --version` with the error kind DisplayVersion\n\
             the guide `show-books` step 1: clap refuses the command `shelf books --help` with the error kind DisplayHelp"
        )
    );
}

#[test]
fn a_guide_of_twelve_steps_passes_and_one_of_thirteen_fails() {
    const STEPS: &[Step] = &[Step::new("List.", &[&["books", "list"]]); 13];
    const STEPS_12: &[Step] = &[Step::new("List.", &[&["books", "list"]]); 12];
    const OK: Tool = Tool::new().help(Help::new().topics(&[Topic::guide("many", "", STEPS_12)]));
    const OVER: Tool = Tool::new().help(Help::new().topics(&[Topic::guide("many", "", STEPS)]));
    assert_eq!(problems(&shelf_with_arguments(), &OK), None);
    assert_eq!(
        problems(&shelf_with_arguments(), &OVER).unwrap(),
        format!(
            "{HEAD}the guide `many` has 13 steps, over the limit of 12; split it into two guides"
        )
    );
}

#[test]
fn a_step_of_three_commands_passes_and_one_of_four_fails() {
    const LIST: &[&str] = &["books", "list"];
    const OK: Tool = Tool::new().help(Help::new().topics(&[Topic::guide(
        "g",
        "",
        &[Step::new("List thrice.", &[LIST, LIST, LIST])],
    )]));
    const OVER: Tool = Tool::new().help(Help::new().topics(&[Topic::guide(
        "g",
        "",
        &[Step::new("List four times.", &[LIST, LIST, LIST, LIST])],
    )]));
    assert_eq!(problems(&shelf_with_arguments(), &OK), None);
    assert_eq!(
        problems(&shelf_with_arguments(), &OVER).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1 has 4 commands, over the limit of 3; split it into two steps"
        )
    );
}

#[test]
fn check_help_fails_a_step_without_commands_and_a_guide_without_steps() {
    const NONE: Tool = Tool::new().help(Help::new().topics(&[
        Topic::guide("empty", "", &[]),
        Topic::guide("bare", "", &[Step::new("Think.", &[])]),
    ]));
    assert_eq!(
        problems(&shelf(), &NONE).unwrap(),
        format!(
            "{HEAD}the guide `empty` has no steps, so `shelf help empty` prints nothing\n\
             the guide `bare` step 1 has no commands, so it shows nothing to run"
        )
    );
}

#[test]
fn check_help_fails_an_empty_explanation_and_one_that_holds_a_line_break() {
    const BAD: Topic = Topic::guide(
        "g",
        "",
        &[
            Step::new(" ", &[&["books", "list"]]),
            Step::new("List\nthem.", &[&["books", "list"]]),
        ],
    );
    const TOOL: Tool = Tool::new().help(Help::new().topics(&[BAD]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1 has an empty explanation\n\
             the guide `g` step 2 has an explanation with a line break, so help cannot print it on one line: `List\\nthem.`"
        )
    );
}

#[test]
fn a_guide_still_follows_the_topic_name_rules() {
    const TOOL: Tool = Tool::new().help(Help::new().topics(&[Topic::guide(
        "books",
        "",
        &[Step::new("List.", &[&["books", "list"]])],
    )]));
    assert!(
        problems(&shelf_with_arguments(), &TOOL)
            .unwrap()
            .contains("the topic `books` is a name of the command")
    );
}
