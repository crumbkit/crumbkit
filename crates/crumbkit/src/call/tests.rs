//! Tests for calls and how they render against a command tree.

use clap::{CommandFactory as _, Parser, Subcommand};

use super::{Call, CallError};

/// A tool with a visible branch, a hidden command and a hidden argument.
#[derive(Parser)]
#[command(name = "shelf")]
struct Tool {
    #[command(subcommand)]
    resource: Resource,
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Resource {
    Books {
        #[command(subcommand)]
        command: Books,
    },
    #[command(hide = true)]
    Admin,
}

#[derive(Subcommand)]
enum Books {
    List {
        #[arg(long)]
        author: Option<String>,
    },
    Show {
        #[arg(required = true, num_args = 1..=100, value_name = "ID")]
        ids: Vec<u32>,
        #[arg(long, hide = true)]
        trace: bool,
    },
}

/// Renders `call` against the tool's command tree.
fn render(call: &Call) -> Result<String, CallError> {
    call.render(&mut Tool::command())
}

#[test]
fn a_call_renders_the_tool_name_the_path_and_a_positional_that_takes_many_values() {
    let call = Call::new(&["books", "show"]).arg("ids");

    assert_eq!(render(&call).unwrap(), "shelf books show <ID>...");
}

#[test]
fn a_call_renders_an_option_with_its_value_name() {
    let call = Call::new(&["books", "list"]).arg("author");

    assert_eq!(render(&call).unwrap(), "shelf books list --author <AUTHOR>");
}

#[test]
fn a_call_with_no_arguments_renders_the_tool_name_and_the_path() {
    let call = Call::new(&["books", "list"]);

    assert_eq!(render(&call).unwrap(), "shelf books list");
}

#[test]
fn a_call_finds_a_global_argument_on_the_subcommand_it_names() {
    let call = Call::new(&["books", "list"]).arg("json");

    assert_eq!(render(&call).unwrap(), "shelf books list --json");
}

#[test]
fn a_path_segment_that_names_no_subcommand_is_a_missing_command() {
    let error = render(&Call::new(&["books", "shw"])).unwrap_err();

    assert_eq!(
        error,
        CallError::MissingCommand {
            command: "shelf books".to_owned(),
            name: "shw".to_owned(),
        }
    );
    assert_eq!(error.to_string(), "'shelf books' has no subcommand 'shw'");
}

#[test]
fn a_path_segment_that_names_a_hidden_subcommand_is_a_hidden_command() {
    let error = render(&Call::new(&["admin"])).unwrap_err();

    assert_eq!(
        error,
        CallError::HiddenCommand {
            command: "shelf".to_owned(),
            name: "admin".to_owned(),
        }
    );
    assert_eq!(error.to_string(), "'shelf' hides its subcommand 'admin'");
}

#[test]
fn an_argument_id_that_the_final_command_lacks_is_a_missing_argument() {
    let error = render(&Call::new(&["books", "list"]).arg("ids")).unwrap_err();

    assert_eq!(
        error,
        CallError::MissingArgument {
            command: "shelf books list".to_owned(),
            id: "ids".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "'shelf books list' has no argument 'ids'"
    );
}

#[test]
fn an_argument_id_that_names_a_hidden_argument_is_a_hidden_argument() {
    let error = render(&Call::new(&["books", "show"]).arg("trace")).unwrap_err();

    assert_eq!(
        error,
        CallError::HiddenArgument {
            command: "shelf books show".to_owned(),
            id: "trace".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "'shelf books show' hides its argument 'trace'"
    );
}
