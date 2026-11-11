//! Tests for the query rule of the mount check.

use clap::{Arg, ArgAction, Args, Command};
use crumbkit::commands::CommandsList;
use crumbkit::queries::QUERIES;
use crumbkit::schemas::{SchemasList, SchemasShow};
use crumbkit::{NoInput, Page, Tool, ViewFlags};

use super::fix;
use crate::check_mounts;
use crate::testing::panic_message;

const TOOL: Tool = Tool::new();

/// A `list` command that takes crumbkit's page flags.
fn list() -> Command {
    <Page as Args>::augment_args(Command::new("list"))
}

/// `commands list` with crumbkit's arguments.
fn commands() -> Command {
    Command::new("commands").subcommand(CommandsList::augment_args(Command::new("list")))
}

/// `schemas list` and `schemas show` with crumbkit's arguments.
fn schemas() -> Command {
    Command::new("schemas")
        .subcommand(SchemasList::augment_args(Command::new("list")))
        .subcommand(SchemasShow::augment_args(Command::new("show")))
}

/// `self figures list`, `self usage` and `self runs list`.
fn usage() -> Command {
    Command::new("self")
        .subcommand(Command::new("figures").subcommand(list()))
        .subcommand(Command::new("usage"))
        .subcommand(Command::new("runs").subcommand(list()))
}

/// `root` with `--json`, `--full` and `--no-input` as crumbkit's types add
/// them.
fn flagged(root: Command) -> Command {
    NoInput::augment_args(ViewFlags::augment_args(root))
}

#[test]
fn a_tool_that_takes_every_flag_of_every_query_passes() {
    let shelf = flagged(Command::new("shelf"))
        .subcommand(commands())
        .subcommand(schemas())
        .subcommand(usage());

    check_mounts(&shelf, &TOOL);
}

#[test]
fn a_tool_that_mounts_none_of_the_commands_passes() {
    let shelf = Command::new("shelf").subcommand(Command::new("books").subcommand(list()));

    check_mounts(&shelf, &TOOL);
}

#[test]
fn a_tool_without_no_input_fails_and_names_the_type_to_flatten() {
    let shelf = ViewFlags::augment_args(Command::new("shelf")).subcommand(commands());

    let message = panic_message(|| check_mounts(&shelf, &TOOL)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's mount rules:\n\
         shelf commands list: a program that reads the tool runs `shelf commands list --json --full --all --no-input`, and the command does not accept `--no-input`, so flatten `crumbkit::NoInput` into the tool's root arguments"
    );
}

#[test]
fn a_tool_without_the_view_flags_fails_and_names_json_and_full() {
    let shelf = NoInput::augment_args(Command::new("shelf")).subcommand(commands());

    let message = panic_message(|| check_mounts(&shelf, &TOOL)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's mount rules:\n\
         shelf commands list: a program that reads the tool runs `shelf commands list --json --full --all --no-input`, and the command does not accept `--json` and `--full`, so flatten `crumbkit::ViewFlags` into the tool's root arguments"
    );
}

#[test]
fn a_list_without_the_page_flags_fails_and_names_all() {
    let shelf = flagged(Command::new("shelf")).subcommand(
        Command::new("self").subcommand(Command::new("runs").subcommand(Command::new("list"))),
    );

    let message = panic_message(|| check_mounts(&shelf, &TOOL)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's mount rules:\n\
         shelf self runs list: a program that reads the tool runs `shelf self runs list --json --full --all --no-input`, and the command does not accept `--all`, so flatten `crumbkit::Page` into the command's arguments"
    );
}

#[test]
fn a_query_that_lacks_the_flags_of_two_types_names_both_on_one_line() {
    let shelf = Command::new("shelf").subcommand(schemas());

    let message = panic_message(|| check_mounts(&shelf, &TOOL)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's mount rules:\n\
         shelf schemas list: a program that reads the tool runs `shelf schemas list --json --full --all --no-input`, and the command does not accept `--json` and `--full`, so flatten `crumbkit::ViewFlags` into the tool's root arguments; it does not accept `--no-input`, so flatten `crumbkit::NoInput` into the tool's root arguments\n\
         shelf schemas show: a program that reads the tool runs `shelf schemas show --json --no-input -- crumbkit/schemas-list/v1`, and the command does not accept `--json`, so flatten `crumbkit::ViewFlags` into the tool's root arguments; it does not accept `--no-input`, so flatten `crumbkit::NoInput` into the tool's root arguments"
    );
}

#[test]
fn a_cargo_subcommand_is_read_below_its_own_name() {
    let cargo = ViewFlags::augment_args(Command::new("cargo"))
        .subcommand(Command::new("crumbkit").subcommand(commands()));

    let message = panic_message(|| check_mounts(&cargo, &TOOL)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's mount rules:\n\
         cargo crumbkit commands list: a program that reads the tool runs `cargo crumbkit commands list --json --full --all --no-input`, and the command does not accept `--no-input`, so flatten `crumbkit::NoInput` into the tool's root arguments"
    );
}

#[test]
fn a_query_that_clap_refuses_with_every_flag_present_fails_with_clap_s_error() {
    let shelf = flagged(Command::new("shelf"))
        .arg(
            Arg::new("profile")
                .long("profile")
                .required(true)
                .action(ArgAction::Set),
        )
        .subcommand(commands());

    let message = panic_message(|| check_mounts(&shelf, &TOOL)).unwrap();

    assert_eq!(
        message,
        "the command tree breaks crumbkit's mount rules:\n\
         shelf commands list: a program that reads the tool runs `shelf commands list --json --full --all --no-input`, and clap refuses it with the error `MissingRequiredArgument`, so make the command accept the query as it is"
    );
}

#[test]
fn every_flag_of_every_query_has_a_type_that_adds_it() {
    for query in QUERIES {
        for flag in query.flags() {
            assert!(
                fix(flag).contains("crumbkit::"),
                "{flag} names no type of crumbkit"
            );
        }
    }
}
