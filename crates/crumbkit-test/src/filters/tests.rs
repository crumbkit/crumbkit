//! Tests for the filters check.

use clap::{Arg, ArgAction, Args as _, Command};
use crumbkit::Output;

use super::check_filters;
use crate::testing::panic_message;

#[test]
fn crumbkits_output_flags_and_a_filter_that_takes_repeats_and_commas_pass() {
    let shelf = Output::augment_args(Command::new("shelf")).subcommand(
        Command::new("books").subcommand(
            Command::new("list")
                .arg(
                    Arg::new("author")
                        .long("author")
                        .value_delimiter(',')
                        .action(ArgAction::Append),
                )
                .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
                .arg(Arg::new("words").num_args(1..).action(ArgAction::Set)),
        ),
    );

    check_filters(&shelf, &[]);
}

#[test]
fn a_repeatable_argument_without_a_comma_delimiter_fails() {
    let list = Command::new("list")
        .arg(Arg::new("author").long("author").action(ArgAction::Append))
        .arg(
            Arg::new("year")
                .long("year")
                .value_delimiter(';')
                .action(ArgAction::Append),
        );

    assert_eq!(
        panic_message(|| check_filters(&list, &[])).unwrap(),
        "the command tree breaks crumbkit's filter rules:\n\
         list: the argument `--author` accepts many values but does not accept both repeats and commas, because it has no value delimiter `,`; such an argument uses `ArgAction::Append` and the value delimiter `,`\n\
         list: the argument `--year` accepts many values but does not accept both repeats and commas, because it has no value delimiter `,`; such an argument uses `ArgAction::Append` and the value delimiter `,`"
    );
}

#[test]
fn an_argument_that_takes_several_values_in_one_use_without_repeats_fails() {
    let list = Command::new("list")
        .arg(
            Arg::new("author")
                .long("author")
                .num_args(1..=3)
                .value_delimiter(',')
                .action(ArgAction::Set),
        )
        .arg(
            Arg::new("year")
                .long("year")
                .num_args(1..=3)
                .action(ArgAction::Set),
        );

    assert_eq!(
        panic_message(|| check_filters(&list, &[])).unwrap(),
        "the command tree breaks crumbkit's filter rules:\n\
         list: the argument `--author` accepts many values but does not accept both repeats and commas, because its action is not `ArgAction::Append`; such an argument uses `ArgAction::Append` and the value delimiter `,`\n\
         list: the argument `--year` accepts many values but does not accept both repeats and commas, because its action is not `ArgAction::Append` and it has no value delimiter `,`; such an argument uses `ArgAction::Append` and the value delimiter `,`"
    );
}

#[test]
fn a_filter_that_takes_repeats_and_commas_and_several_values_in_one_use_fails() {
    let list = Command::new("list")
        .arg(
            Arg::new("author")
                .long("author")
                .num_args(1..=3)
                .value_delimiter(',')
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("year")
                .long("year")
                .num_args(1..)
                .value_delimiter(',')
                .action(ArgAction::Append),
        );

    assert_eq!(
        panic_message(|| check_filters(&list, &[])).unwrap(),
        "the command tree breaks crumbkit's filter rules:\n\
         list: the argument `--author` takes several values in one use through its `num_args`, and a filter takes one value per use\n\
         list: the argument `--year` takes several values in one use through its `num_args`, and a filter takes one value per use"
    );
}
