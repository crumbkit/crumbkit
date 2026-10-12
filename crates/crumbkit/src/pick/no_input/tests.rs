//! Tests for `--no-input`.

use clap::{Args as _, Command, Parser, Subcommand};

use super::{NO_INPUT_ID, NoInput};

/// A tool that takes `--no-input` at its root.
#[derive(Parser)]
#[command(name = "tool")]
struct Tool {
    #[command(subcommand)]
    command: Commands,
    #[command(flatten)]
    no_input: NoInput,
}

#[derive(Subcommand)]
enum Commands {
    Remove,
}

#[test]
fn no_input_is_a_global_flag_that_a_subcommand_accepts_after_its_name() {
    let tool = Tool::try_parse_from(["tool", "remove", "--no-input"]).unwrap();
    assert_eq!(tool.no_input, NoInput { no_input: true });
}

#[test]
fn the_flag_has_crumbkits_id_and_the_long_name_no_input() {
    let command = NoInput::augment_args(Command::new("tool"));
    let arg = command
        .get_arguments()
        .find(|arg| arg.get_id() == NO_INPUT_ID)
        .unwrap();
    assert_eq!(arg.get_long(), Some("no-input"));
    assert!(arg.is_global_set());
}

#[test]
fn the_flag_serializes_as_an_object_with_no_input() {
    let json = serde_json::to_string(&NoInput::default()).unwrap();
    assert_eq!(json, r#"{"no_input":false}"#);
}
