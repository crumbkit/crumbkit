//! Tests for the `value_hint` field of an argument in `commands list`.

use std::path::PathBuf;

use clap::builder::ValueHint;
use clap::{Arg, ArgAction, Args, Command, value_parser};
use serde_json::{Value, json};

use super::{CommandRow, Tool};

/// Arguments as a tool's derive declares them, one of which narrows the
/// hint of its `PathBuf`.
#[derive(Args)]
struct Copy {
    /// A path with no hint of its own.
    #[arg(long)]
    from: PathBuf,
    /// A path whose hint says it names a directory.
    #[arg(long, value_hint = ValueHint::DirPath)]
    to: PathBuf,
    /// A value that is not a path.
    #[arg(long)]
    label: String,
    /// A flag, which takes no value.
    #[arg(long)]
    force: bool,
}

/// Returns the row of `command` after clap's build, as JSON.
fn row(mut command: Command) -> Value {
    command.build();
    serde_json::to_value(CommandRow::new(
        "shelf",
        &["copy"],
        &command,
        None,
        &Tool::new(),
        false,
    ))
    .unwrap()
}

/// Returns the `value_hint` of each argument of `command`, in order.
fn hints(command: Command) -> Value {
    row(command)["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| argument["value_hint"].clone())
        .collect()
}

#[test]
fn clap_reports_any_path_for_a_path_buf_argument_that_sets_no_hint() {
    let mut command = Copy::augment_args(Command::new("copy"));
    command.build();

    let from = command
        .get_arguments()
        .find(|arg| arg.get_id() == "from")
        .unwrap();

    assert_eq!(from.get_value_hint(), ValueHint::AnyPath);
}

#[test]
fn a_derived_argument_reports_any_path_its_narrowed_hint_or_null() {
    assert_eq!(
        hints(Copy::augment_args(Command::new("copy"))),
        json!(["any-path", "dir-path", null, null])
    );
}

#[test]
fn a_path_buf_value_parser_without_derive_reports_any_path() {
    let command = Command::new("copy").arg(
        Arg::new("from")
            .long("from")
            .value_parser(value_parser!(PathBuf)),
    );

    assert_eq!(hints(command), json!(["any-path"]));
}

#[test]
fn a_flag_and_a_string_argument_report_a_null_value_hint() {
    let command = Command::new("copy")
        .arg(Arg::new("label").long("label"))
        .arg(Arg::new("force").long("force").action(ArgAction::SetTrue));

    assert_eq!(hints(command), json!([null, null]));
}

#[test]
fn every_value_hint_prints_as_the_kebab_case_of_clap_s_name() {
    let names = [
        (ValueHint::Other, "other"),
        (ValueHint::AnyPath, "any-path"),
        (ValueHint::FilePath, "file-path"),
        (ValueHint::DirPath, "dir-path"),
        (ValueHint::ExecutablePath, "executable-path"),
        (ValueHint::CommandName, "command-name"),
        (ValueHint::CommandString, "command-string"),
        (ValueHint::Username, "username"),
        (ValueHint::Hostname, "hostname"),
        (ValueHint::Url, "url"),
        (ValueHint::EmailAddress, "email-address"),
    ];
    for (hint, name) in names {
        let command = Command::new("copy").arg(Arg::new("value").long("value").value_hint(hint));

        assert_eq!(hints(command), json!([name]), "{hint:?}");
    }
}

#[test]
fn a_command_with_arguments_hint_prints_in_kebab_case() {
    let command = Command::new("copy").trailing_var_arg(true).arg(
        Arg::new("command")
            .num_args(1..)
            .value_hint(ValueHint::CommandWithArguments),
    );

    assert_eq!(hints(command), json!(["command-with-arguments"]));
}
