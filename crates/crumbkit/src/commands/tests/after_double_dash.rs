//! Tests for the `after_double_dash` field of an argument in `commands list`.

use clap::{Arg, Command};
use serde_json::{Value, json};

use super::{CommandRow, Tool};

/// Returns the `after_double_dash` of each argument of `command`, in order.
fn flags(mut command: Command) -> Value {
    command.build();
    let row = serde_json::to_value(CommandRow::new(
        "starmap",
        &["run"],
        &command,
        None,
        &Tool::new(),
        false,
    ))
    .unwrap();
    row["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| argument["after_double_dash"].clone())
        .collect()
}

#[test]
fn a_positional_argument_set_last_reports_true_and_the_others_report_false() {
    let command = Command::new("run")
        .arg(Arg::new("env").long("env"))
        .arg(Arg::new("name"))
        .arg(Arg::new("program").num_args(1..).last(true));

    assert_eq!(flags(command), json!([false, false, true]));
}

#[test]
fn a_trailing_var_arg_positional_that_is_not_last_reports_false() {
    let command = Command::new("run")
        .trailing_var_arg(true)
        .arg(Arg::new("program").num_args(1..));

    assert_eq!(flags(command), json!([false]));
}
