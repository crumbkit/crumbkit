//! Tests for the tool record check of guide topics.

use clap::{Args as _, Command};
use crumbkit::help::{Example, Help, Step, Topic};
use crumbkit::{Document, Prose, Tool, ViewFlags};

use super::{LEAVES, check_help, panic_message, shelf, shelf_with_arguments};
use crumbkit::help::Leaf;

/// The examples of the two leaves of `shelf_with_arguments`.
const BOOKS: &[Leaf] = &[
    Leaf::new(&["books", "list"]).examples(&[Example::new(&[], "List")]),
    Leaf::new(&["books", "show"]).examples(&[Example::new(&["1"], "Show")]),
];

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
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[SHOW]));
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
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[BAD]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `show-books` step 1: clap refuses the command `shelf books list --everything` with the error kind UnknownArgument"
        )
    );
}

#[test]
fn check_help_names_the_step_number_of_the_command_that_clap_refuses() {
    const BAD: Topic = Topic::guide(
        "show-books",
        "",
        &[
            Step::new("List.", &[&["books", "list"]]),
            Step::new("Show.", &[&["books", "show", "1"], &["books", "show"]]),
        ],
    );
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[BAD]));
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
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[BAD]));
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
    const OK: Tool = Tool::new().help(
        Help::new()
            .leaves(BOOKS)
            .topics(&[Topic::guide("many", "", STEPS_12)]),
    );
    const OVER: Tool = Tool::new().help(
        Help::new()
            .leaves(BOOKS)
            .topics(&[Topic::guide("many", "", STEPS)]),
    );
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
    const OK: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[Topic::guide(
        "g",
        "",
        &[Step::new("List thrice.", &[LIST, LIST, LIST])],
    )]));
    const OVER: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[Topic::guide(
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
    const NONE: Tool = Tool::new().help(Help::new().leaves(LEAVES).topics(&[
        Topic::guide("empty", "", &[]),
        Topic::guide("bare", "", &[Step::new("Think.", &[])]),
    ]));
    assert_eq!(
        problems(&shelf(), &NONE).unwrap(),
        format!(
            "{HEAD}the guide `empty` has no steps, so `shelf help empty` prints an empty page\n\
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
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[BAD]));
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
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[Topic::guide(
        "books",
        "",
        &[Step::new("List.", &[&["books", "list"]])],
    )]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the topic `books` is a name of the command `shelf books`, so `shelf help books` prints that command's help"
        )
    );
}

/// Asserts that a tool record with the guide topic `$guide` and the `books`
/// leaves passes.
macro_rules! passes {
    ($guide:expr) => {{
        const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[$guide]));
        assert_eq!(problems(&shelf_with_arguments(), &TOOL), None);
    }};
}

#[test]
fn a_guide_with_one_line_explanations_and_a_command_in_each_step_passes() {
    passes!(Topic::guide(
        "g",
        "",
        &[
            Step::new("List.", &[&["books", "list"]]),
            Step::new("List them again.", &[&["books", "list"]]),
        ],
    ));
}

#[test]
fn a_guide_step_with_two_commands_that_parse_passes() {
    passes!(Topic::guide(
        "g",
        "",
        &[Step::new(
            "List.",
            &[&["books", "list"], &["books", "show", "1"]],
        )],
    ));
}

#[test]
fn a_guide_command_without_help_or_version_passes() {
    passes!(Topic::guide(
        "g",
        "",
        &[Step::new("Ask.", &[&["books", "list"]])],
    ));
}

#[test]
fn a_guide_command_that_names_a_group_without_a_required_subcommand_passes() {
    // clap parses `shelf books` and prints help at run time, and the check
    // names no leaf for it, so it applies no view-flag rule.
    passes!(Topic::guide("g", "", &[Step::new("Look.", &[&["books"]])]));
}

/// Leaves whose `books list` has a document, so that it offers `--long`
/// only when its document says so, and `books show` has none.
const DOCUMENTED: &[Leaf] = &[
    Leaf::new(&["books", "list"])
        .examples(&[Example::new(&[], "List")])
        .document(Document::<Prose<String>>::new(
            "shelf",
            "count",
            1,
            "The count.",
        )),
    Leaf::new(&["books", "show"]).examples(&[Example::new(&["1"], "Show")]),
];

/// Runs `check_help` over `shelf_with_arguments` with the view flags and a
/// guide of one step with the command `words`.
fn view_problems(words: &'static [&'static str]) -> Option<String> {
    // A test leaks the small slices that a `'static` tool record borrows.
    let steps: &'static [Step] = Box::leak(Box::new([Step::new(
        "Count.",
        Box::leak(Box::new([words])),
    )]));
    let guide: &'static [Topic] = Box::leak(Box::new([Topic::guide("g", "", steps)]));
    let tool = Tool::new().help(Help::new().leaves(DOCUMENTED).topics(guide));
    let tree = ViewFlags::augment_args(shelf_with_arguments());
    problems(&tree, &tool)
}

#[test]
fn check_help_fails_a_guide_command_that_types_a_view_flag_its_leaf_does_not_offer() {
    assert_eq!(
        view_problems(&["books", "list", "--long"]).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1: the command `shelf books list --long` types --long, which the command does not offer, so crumbkit refuses it"
        )
    );
}

#[test]
fn a_guide_command_that_types_only_the_json_flag_passes() {
    assert_eq!(view_problems(&["books", "list", "--json"]), None);
}

#[test]
fn check_help_prints_a_refused_command_with_its_words_quoted() {
    const BAD: Topic = Topic::guide(
        "g",
        "",
        &[Step::new(
            "Show.",
            &[&["books", "show", "--title", "Sense and Sensibility"]],
        )],
    );
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[BAD]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1: clap refuses the command `shelf books show --title 'Sense and Sensibility'` with the error kind UnknownArgument"
        )
    );
}

#[test]
fn check_help_fails_a_command_word_with_a_line_break_and_passes_the_word_without_one() {
    const BAD: Topic = Topic::guide(
        "g",
        "",
        &[Step::new("Show.", &[&["books", "show", "a\nb"]])],
    );
    const GOOD: Topic = Topic::guide("g", "", &[Step::new("Show.", &[&["books", "show", "a b"]])]);
    const BAD_TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[BAD]));
    const GOOD_TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[GOOD]));
    assert_eq!(
        problems(&shelf_with_arguments(), &BAD_TOOL).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1 has a command word with a line break, so help cannot print the command on one line: `a\\nb`"
        )
    );
    assert_eq!(problems(&shelf_with_arguments(), &GOOD_TOOL), None);
}

#[test]
fn a_step_that_points_to_another_tool_s_topic_passes() {
    passes!(Topic::guide(
        "g",
        "",
        &[
            Step::see("Sign in with the login tool.", "logintool", "sign-in"),
            Step::new("List.", &[&["books", "list"]]),
        ],
    ));
}

#[test]
fn a_pointer_step_has_no_commands_and_the_command_limit_does_not_apply() {
    passes!(Topic::guide(
        "g",
        "",
        &[Step::see("Sign in.", "logintool", "sign-in")]
    ));
}

#[test]
fn a_pointer_step_counts_toward_the_limit_of_twelve_steps() {
    const OK_STEPS: &[Step] = &[Step::see("Sign in.", "logintool", "sign-in"); 12];
    const OVER_STEPS: &[Step] = &[Step::see("Sign in.", "logintool", "sign-in"); 13];
    const OVER: Tool = Tool::new().help(
        Help::new()
            .leaves(BOOKS)
            .topics(&[Topic::guide("many", "", OVER_STEPS)]),
    );
    passes!(Topic::guide("many", "", OK_STEPS));
    assert_eq!(
        problems(&shelf_with_arguments(), &OVER).unwrap(),
        format!(
            "{HEAD}the guide `many` has 13 steps, over the limit of 12; split it into two guides"
        )
    );
}

#[test]
fn check_help_fails_a_pointer_whose_tool_is_not_a_command_name() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[Topic::guide(
        "g",
        "",
        &[
            Step::see("Sign in.", "Login Tool", "sign-in"),
            Step::see("Sign in.", "login-tool", "sign-in"),
        ],
    )]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1 points to the tool `Login Tool`, which is not lowercase letters and digits in words joined by single dashes, starting with a letter"
        )
    );
}

#[test]
fn check_help_fails_a_pointer_whose_topic_is_not_a_topic_name() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[Topic::guide(
        "g",
        "",
        &[
            Step::see("Sign in.", "logintool", "Sign_in"),
            Step::see("Sign in.", "logintool", "sign-in-2"),
        ],
    )]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1 points to the topic `Sign_in`, which is not lowercase letters and digits in words joined by single dashes, starting with a letter"
        )
    );
}

#[test]
fn check_help_fails_a_pointer_to_the_tool_s_own_name() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[Topic::guide(
        "g",
        "",
        &[
            Step::see("Sign in.", "shelf", "sign-in"),
            Step::see("Sign in.", "shelves", "sign-in"),
        ],
    )]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!(
            "{HEAD}the guide `g` step 1 points to the tool `shelf`, which is this tool's own name, so use a step with commands for a topic of this tool"
        )
    );
}

#[test]
fn check_help_still_checks_a_pointer_step_s_explanation() {
    const TOOL: Tool = Tool::new().help(Help::new().leaves(BOOKS).topics(&[Topic::guide(
        "g",
        "",
        &[Step::see(" ", "logintool", "sign-in")],
    )]));
    assert_eq!(
        problems(&shelf_with_arguments(), &TOOL).unwrap(),
        format!("{HEAD}the guide `g` step 1 has an empty explanation")
    );
}
