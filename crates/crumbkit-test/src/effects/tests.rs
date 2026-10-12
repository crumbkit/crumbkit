//! Tests for the effects check.

use clap::{Arg, ArgAction, Command};
use crumbkit::help::{Effects, Guide, Leaf};

use super::check_effects;
use crate::allow::{Allow, Rule};
use crate::testing::panic_message;

/// Returns crumbkit's `--dry-run`, with the id that `crumbkit::DryRun`
/// gives it.
fn crumbkit_dry_run() -> Arg {
    Arg::new("crumbkit_dry_run")
        .long("dry-run")
        .action(ArgAction::SetTrue)
}

/// Returns a tool's own `--dry-run`.
fn own_dry_run() -> Arg {
    Arg::new("dry_run")
        .long("dry-run")
        .action(ArgAction::SetTrue)
}

/// Returns a `--yes` flag.
fn yes() -> Arg {
    Arg::new("yes").long("yes").action(ArgAction::SetTrue)
}

/// Returns crumbkit's `--no-input`, with the id that `crumbkit::NoInput`
/// gives it.
fn no_input() -> Arg {
    Arg::new("crumbkit_no_input")
        .long("no-input")
        .global(true)
        .action(ArgAction::SetTrue)
}

/// Returns `shelf books` with `list`, and `remove` with `args`, and
/// crumbkit's `--no-input` at the root.
fn shelf(remove: impl IntoIterator<Item = Arg>) -> Command {
    Command::new("shelf").arg(no_input()).subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("remove").args(remove)),
    )
}

const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["books", "list"]).effects(Effects::read_only()),
    Leaf::new(&["books", "remove"]).effects(Effects::destructive().idempotent()),
]);

#[test]
fn leaves_that_declare_effects_and_a_destructive_leaf_with_dry_run_and_yes_pass() {
    check_effects(&shelf([crumbkit_dry_run(), yes()]), &GUIDE, &[]);
}

#[test]
fn a_visible_leaf_without_effects_fails_and_a_hidden_one_passes() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"])]);
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("show"))
            .subcommand(Command::new("debug").hide(true)),
    );

    assert_eq!(
        panic_message(|| check_effects(&shelf, &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's effects rules:\n\
         shelf books list: the command declares no effects, so an agent must assume it may destroy data\n\
         shelf books show: the command declares no effects, so an agent must assume it may destroy data"
    );
}

#[test]
fn a_leaf_without_effects_passes_when_an_opt_out_names_it() {
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["books", "list"]).effects(Effects::read_only())]);
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::Effects,
        "books remove",
        "The command's effects depend on a plugin.",
    )];

    check_effects(&shelf([]), &GUIDE, ALLOW);
}

#[test]
fn crumbkit_s_dry_run_on_a_read_only_leaf_fails() {
    let shelf = Command::new("shelf").arg(no_input()).subcommand(
        Command::new("books")
            .subcommand(Command::new("list").arg(crumbkit_dry_run()))
            .subcommand(Command::new("remove").arg(yes())),
    );

    assert_eq!(
        panic_message(|| check_effects(&shelf, &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's effects rules:\n\
         shelf books list: the command is read-only and takes crumbkit's `--dry-run`, which has nothing to skip"
    );
}

#[test]
fn a_tool_s_own_dry_run_fails_anywhere_in_the_tree() {
    let shelf = shelf([own_dry_run(), yes()]).arg(own_dry_run());

    assert_eq!(
        panic_message(|| check_effects(&shelf, &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's effects rules:\n\
         shelf: the command has its own `--dry-run`, and crumbkit's `DryRun` gives that flag one meaning in every tool\n\
         shelf books remove: the command has its own `--dry-run`, and crumbkit's `DryRun` gives that flag one meaning in every tool"
    );
}

#[test]
fn a_command_keeps_its_own_dry_run_when_an_opt_out_names_it() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::DryRun,
        "books",
        "The tool's scripts pass `--dry-run` today.",
    )];

    check_effects(&shelf([own_dry_run(), yes()]), &GUIDE, ALLOW);
}

#[test]
fn a_destructive_leaf_without_yes_fails_unless_an_opt_out_names_it() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::Yes,
        "books remove",
        "The command removes only the tool's cache.",
    )];

    assert_eq!(
        panic_message(|| check_effects(&shelf([crumbkit_dry_run()]), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's effects rules:\n\
         shelf books remove: the command is destructive and has no `--yes`, so an agent without a terminal cannot confirm it"
    );
    check_effects(&shelf([crumbkit_dry_run()]), &GUIDE, ALLOW);
}

#[test]
fn an_opt_out_from_an_effects_rule_that_excuses_nothing_fails_the_check() {
    const ALLOW: &[Allow] = &[
        Allow::new(Rule::Effects, "books list", "Old habit."),
        Allow::new(Rule::Yes, "books shelve", "Old habit."),
    ];

    assert_eq!(
        panic_message(|| check_effects(&shelf([yes()]), &GUIDE, ALLOW)).unwrap(),
        "the command tree breaks crumbkit's effects rules:\n\
         the opt-out from `effects/declared` for `shelf books list` excuses no problem, so it is stale\n\
         the opt-out from `effects/yes` for `shelf books shelve` names no command"
    );
}

#[test]
fn a_root_without_subcommands_is_a_leaf() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&[]).effects(Effects::read_only())]);

    check_effects(&Command::new("shelf"), &GUIDE, &[]);
    assert_eq!(
        panic_message(|| check_effects(&Command::new("shelf"), &Guide::new(), &[])).unwrap(),
        "the command tree breaks crumbkit's effects rules:\n\
         shelf: the command declares no effects, so an agent must assume it may destroy data"
    );
}

/// A picker that picks nothing.
struct Nobody;

impl crumbkit::pick::Picker for Nobody {
    fn pick(
        &self,
        _question: &crumbkit::pick::Question<'_>,
        _input: &mut dyn std::io::BufRead,
        _output: &mut dyn std::io::Write,
    ) -> std::io::Result<crumbkit::pick::Answer> {
        Ok(crumbkit::pick::Answer::Cancelled)
    }
}

static NOBODY: Nobody = Nobody;

/// Returns `shelf books` with `list` and `remove`, without `--no-input`.
fn without_no_input(remove: impl IntoIterator<Item = Arg>) -> Command {
    Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Command::new("list"))
            .subcommand(Command::new("remove").args(remove)),
    )
}

#[test]
fn a_tool_with_yes_and_without_crumbkit_s_no_input_fails_unless_an_opt_out_names_the_root() {
    const ALLOW: &[Allow] = &[Allow::new(
        Rule::NoInput,
        "",
        "Only scripts run the tool, and they pass `--yes`.",
    )];

    assert_eq!(
        panic_message(|| check_effects(&without_no_input([yes()]), &GUIDE, &[])).unwrap(),
        "the command tree breaks crumbkit's effects rules:\n\
         shelf: the tool can ask a question and has no `--no-input` from crumbkit's `NoInput`, so a caller cannot say that nobody can answer"
    );
    check_effects(&without_no_input([yes()]), &GUIDE, ALLOW);
}

#[test]
fn a_tool_with_a_picker_needs_crumbkit_s_no_input() {
    const GUIDE: Guide = Guide::new().picker(&NOBODY).leaves(&[
        Leaf::new(&["books", "list"]).effects(Effects::read_only()),
        Leaf::new(&["books", "remove"]).effects(Effects::additive()),
    ]);

    assert!(panic_message(|| check_effects(&without_no_input([]), &GUIDE, &[])).is_some());
    check_effects(&shelf([]), &GUIDE, &[]);
}

#[test]
fn a_tool_that_never_asks_needs_no_no_input() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "list"]).effects(Effects::read_only()),
        Leaf::new(&["books", "remove"]).effects(Effects::additive()),
    ]);

    check_effects(&without_no_input([]), &GUIDE, &[]);
}
