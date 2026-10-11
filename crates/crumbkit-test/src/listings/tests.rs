//! Tests for the listings check.

use clap::builder::ValueRange;
use clap::{Arg, ArgAction, ArgGroup, Args as _, Command};
use crumbkit::Page;

use super::check_listings;
use crate::testing::panic_message;

/// Returns a `show` command whose positional `ids` argument is required or
/// not and takes `num_args` values.
fn show(required: bool, num_args: impl Into<ValueRange>) -> Command {
    Command::new("show").arg(
        Arg::new("ids")
            .required(required)
            .num_args(num_args)
            .action(ArgAction::Append),
    )
}

#[test]
fn a_list_command_that_flattens_page_and_a_show_command_that_takes_1_to_100_ids_pass() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("books")
            .subcommand(Page::<4>::augment_args(Command::new("list")))
            .subcommand(show(true, 1..=100)),
    );

    check_listings(&shelf, &[]);
}

#[test]
fn a_show_command_whose_ids_sit_in_a_required_group_passes() {
    let show = show(false, 1..=100)
        .arg(Arg::new("from").long("from"))
        .group(ArgGroup::new("source").args(["ids", "from"]).required(true));

    check_listings(&show, &[]);
}

#[test]
fn a_show_command_whose_ids_sit_only_in_a_group_that_is_not_required_fails() {
    let show = show(false, 1..=100)
        .arg(Arg::new("from").long("from"))
        .group(ArgGroup::new("source").args(["ids", "from"]));

    assert!(panic_message(|| check_listings(&show, &[])).is_some());
}

#[test]
fn a_list_command_without_limit_and_all_fails() {
    let list = Command::new("list");

    assert_eq!(
        panic_message(|| check_listings(&list, &[])).unwrap(),
        "the command tree breaks crumbkit's listing rules:\n\
         list: the command has no `--limit` argument that takes a value, and a `list` command has one, as flattening crumbkit's `Page` gives it\n\
         list: the command has no `--all` flag, and a `list` command has one, as flattening crumbkit's `Page` gives it"
    );
}

#[test]
fn a_list_command_whose_limit_takes_no_value_and_whose_all_takes_one_fails() {
    let list = Command::new("list")
        .arg(Arg::new("limit").long("limit").action(ArgAction::SetTrue))
        .arg(Arg::new("all").long("all").action(ArgAction::Set));

    assert_eq!(
        panic_message(|| check_listings(&list, &[])).unwrap(),
        "the command tree breaks crumbkit's listing rules:\n\
         list: the command has no `--limit` argument that takes a value, and a `list` command has one, as flattening crumbkit's `Page` gives it\n\
         list: the command has no `--all` flag, and a `list` command has one, as flattening crumbkit's `Page` gives it"
    );
}

#[test]
fn a_show_command_without_a_required_positional_argument_for_1_to_100_values_fails() {
    let rule = "the command has no required positional argument that takes from 1 to 100 values and accepts more than 1, and a `show` command has one";
    let shelf = Command::new("shelf")
        .subcommand(Command::new("none").subcommand(Command::new("show")))
        .subcommand(Command::new("optional").subcommand(show(false, 1..=100)))
        .subcommand(Command::new("one").subcommand(show(true, 1)))
        .subcommand(Command::new("unbounded").subcommand(show(true, 1..)))
        .subcommand(Command::new("too-many").subcommand(show(true, 1..=101)))
        .subcommand(Command::new("from-zero").subcommand(show(true, 0..=100)));

    assert_eq!(
        panic_message(|| check_listings(&shelf, &[])).unwrap(),
        format!(
            "the command tree breaks crumbkit's listing rules:\n\
             shelf none show: {rule}\n\
             shelf optional show: {rule}\n\
             shelf one show: {rule}\n\
             shelf unbounded show: {rule}\n\
             shelf too-many show: {rule}\n\
             shelf from-zero show: {rule}"
        )
    );
}
