//! Tests for calls and how they render against a command tree.

use clap::{CommandFactory as _, Parser, Subcommand};

use super::RenderCall as _;
use crate::{Call, CallError};

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
        #[arg(long, value_parser = ["year", "title"], ignore_case = true)]
        sort: Option<String>,
        #[arg(short = 'n')]
        number: Option<u32>,
    },
    Show {
        #[arg(required = true, num_args = 1..=100, value_name = "ID")]
        ids: Vec<u32>,
        #[arg(long, hide = true)]
        trace: bool,
    },
    Find {
        #[arg(allow_hyphen_values = true)]
        title: String,
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

#[test]
fn a_value_renders_an_option_with_the_value_quoted_for_a_shell() {
    let call = Call::new(&["books", "list"]).value("author", "Le Guin");

    assert_eq!(
        render(&call).unwrap(),
        "shelf books list --author 'Le Guin'"
    );
}

#[test]
fn a_value_renders_an_option_without_a_long_name_with_its_short_name() {
    let call = Call::new(&["books", "list"]).value("number", "3");

    assert_eq!(render(&call).unwrap(), "shelf books list -n 3");
}

#[test]
fn each_value_of_a_positional_renders_as_one_word_in_order_among_the_placeholders() {
    let call = Call::new(&["books", "list"])
        .arg("author")
        .value("number", "3");
    let show = Call::new(&["books", "show"])
        .value("ids", "7")
        .value("ids", "9");

    assert_eq!(
        render(&call).unwrap(),
        "shelf books list --author <AUTHOR> -n 3"
    );
    assert_eq!(render(&show).unwrap(), "shelf books show 7 9");
}

#[test]
fn a_value_for_an_id_that_the_final_command_lacks_is_a_missing_argument() {
    let error = render(&Call::new(&["books", "list"]).value("ids", "7")).unwrap_err();

    assert_eq!(
        error,
        CallError::MissingArgument {
            command: "shelf books list".to_owned(),
            id: "ids".to_owned(),
        }
    );
}

#[test]
fn a_value_for_a_hidden_argument_is_a_hidden_argument() {
    let error = render(&Call::new(&["books", "show"]).value("trace", "yes")).unwrap_err();

    assert_eq!(
        error,
        CallError::HiddenArgument {
            command: "shelf books show".to_owned(),
            id: "trace".to_owned(),
        }
    );
}

#[test]
fn a_value_for_a_flag_is_refused_because_the_flag_takes_no_value() {
    let error = render(&Call::new(&["books", "list"]).value("json", "yes")).unwrap_err();

    assert_eq!(
        error,
        CallError::TakesNoValue {
            command: "shelf books list".to_owned(),
            id: "json".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "'shelf books list' takes no value for its argument 'json'"
    );
}

#[test]
fn a_value_outside_the_possible_values_of_its_argument_is_refused() {
    let error = render(&Call::new(&["books", "list"]).value("sort", "yr")).unwrap_err();

    assert_eq!(
        error,
        CallError::UnknownValue {
            command: "shelf books list".to_owned(),
            id: "sort".to_owned(),
            value: "yr".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "'shelf books list' does not accept 'yr' for its argument 'sort'"
    );
}

#[test]
fn a_possible_value_matches_in_any_case_when_its_argument_ignores_case() {
    let call = Call::new(&["books", "list"]).value("sort", "YEAR");

    assert_eq!(render(&call).unwrap(), "shelf books list --sort YEAR");
}

#[test]
fn an_option_value_that_starts_with_a_hyphen_renders_after_an_equals_sign() {
    let call = Call::new(&["books", "list"])
        .value("author", "-x")
        .value("number", "-3");

    assert_eq!(render(&call).unwrap(), "shelf books list --author=-x -n=-3");
}

#[test]
fn a_positional_value_that_starts_with_a_hyphen_is_refused_because_clap_would_read_a_flag() {
    let error = render(&Call::new(&["books", "show"]).value("ids", "-7")).unwrap_err();

    assert_eq!(
        error,
        CallError::HyphenValue {
            command: "shelf books show".to_owned(),
            id: "ids".to_owned(),
            value: "-7".to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "'shelf books show' would read the value '-7' of its argument 'ids' as a flag"
    );
}

#[test]
fn a_positional_value_that_starts_with_a_hyphen_renders_when_its_argument_allows_hyphen_values() {
    let call = Call::new(&["books", "find"]).value("title", "-x");

    assert_eq!(render(&call).unwrap(), "shelf books find -x");
}

#[test]
fn a_single_hyphen_is_a_value_and_renders_as_a_positional() {
    let call = Call::new(&["books", "show"]).value("ids", "-");

    assert_eq!(render(&call).unwrap(), "shelf books show -");
}
