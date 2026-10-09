//! Tests for the listings check.

use clap::builder::ValueRange;
use clap::{Arg, ArgAction, Args as _, Command};
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
fn a_list_command_without_limit_and_all_fails() {
    let list = Command::new("list");

    assert_eq!(
        panic_message(|| check_listings(&list, &[])).unwrap(),
        "the command tree breaks crumbkit's listing rules:\n\
         list: a `list` command has a `--limit` argument that takes a value, as flattening crumbkit's `Page` gives it\n\
         list: a `list` command has an `--all` flag, as flattening crumbkit's `Page` gives it"
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
         list: a `list` command has a `--limit` argument that takes a value, as flattening crumbkit's `Page` gives it\n\
         list: a `list` command has an `--all` flag, as flattening crumbkit's `Page` gives it"
    );
}

#[test]
fn a_show_command_without_a_required_positional_argument_for_1_to_100_values_fails() {
    let rule = "a `show` command has a required positional argument that takes at least 1 and at most 100 values and accepts more than 1";
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
