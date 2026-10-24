//! Tests for the view flags that `finish_to` refuses because the result
//! cannot use them, and for the pairs that `Output::new_to` refuses, for a
//! tool that parses without crumbkit's `parse`.

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::{build_to, finish_with};
use crate::twins::assert_one_word_apart;
use crate::{Document, Prose, Row};

/// A row without long columns.
#[derive(Serialize, JsonSchema)]
struct Moon {
    /// The moon's name.
    name: &'static str,
}

impl Row for Moon {
    const COLUMNS: &'static [&'static str] = &["name"];
}

const MOONS: Document<Vec<Moon>> = Document::new("solar", "moons", 1, "The test rows.");
const COUNT: Document<Prose<String>> = Document::new("solar", "count", 1, "The test count.");

#[test]
fn finish_refuses_long_for_a_row_type_without_long_columns() {
    assert_one_word_apart(&["--long"], &[]);
    let twin = finish_with(&[], MOONS, vec![Moon { name: "Io" }]);
    assert_eq!(twin.code, ExitCode::SUCCESS, "{}", twin.err);

    let finished = finish_with(&["--long"], MOONS, vec![Moon { name: "Io" }]);

    assert_eq!(finished.code, ExitCode::from(2));
    assert_eq!(finished.out, "");
    assert_eq!(
        finished.err,
        "\
error: --long adds the long columns of a table, and this command's rows have none
  help:  use --fields to name the columns to show
  code:  tool::cli::usage
"
    );
}

#[test]
fn finish_refuses_long_without_json_for_a_prose_result() {
    let eight = || Prose::new("Eight planets.".to_owned());
    assert_one_word_apart(&["--long"], &["--json", "--long"]);
    assert_eq!(
        finish_with(&["--json", "--long"], COUNT, eight()).code,
        ExitCode::SUCCESS
    );

    let long = finish_with(&["--long"], COUNT, eight());

    assert_eq!(long.code, ExitCode::from(2));
    assert!(
        long.err.starts_with(
            "error: --long adds the long columns of a table, and this command prints prose\n"
        ),
        "{}",
        long.err
    );
}

#[test]
fn long_and_full_with_json_for_a_prose_result_print_the_json_document_and_a_note() {
    let eight = || Prose::new("Eight planets.".to_owned());
    let json = finish_with(&["--json"], COUNT, eight());

    let long = finish_with(&["--json", "--long"], COUNT, eight());
    let full = finish_with(&["--json", "--full"], COUNT, eight());

    assert_eq!(json.err, "");
    assert_eq!(long.code, ExitCode::SUCCESS, "{}", long.err);
    assert_eq!(long.out, json.out);
    assert_eq!(
        long.err,
        "note: --long adds nothing to --json here; the JSON already holds the whole result.\n"
    );
    assert_eq!(full.code, ExitCode::SUCCESS, "{}", full.err);
    assert_eq!(full.out, json.out);
    assert_eq!(
        full.err,
        "note: --full adds nothing to --json here; the JSON already holds the whole result.\n"
    );
}

#[test]
fn output_new_refuses_fields_with_long_in_crumbkit_s_words() {
    assert_one_word_apart(&["--fields", "name", "--long"], &["--fields", "name"]);
    let twin = build_to(&["--fields", "name"]);
    assert!(twin.result.is_ok(), "{}", twin.err);

    let built = build_to(&["--fields", "name", "--long"]);

    let stop = built.result.unwrap_err();
    assert_eq!(stop.number(), 2);
    assert!(!stop.output().is_json());
    assert_eq!(built.out, "");
    assert_eq!(
        built.err,
        "\
error: --fields prints only the fields it names, so it cannot be used with --long
  help:  drop --long, and name the long columns in --fields
  code:  tool::cli::usage
"
    );
}

#[test]
fn output_new_refuses_fields_with_full_and_full_without_json_in_crumbkit_s_words() {
    let with_fields = build_to(&["--json", "--fields", "name", "--full"]);
    let without_json = build_to(&["--full"]);

    let stop = with_fields.result.unwrap_err();
    assert_eq!(stop.number(), 2);
    assert!(stop.output().is_json());
    let document: serde_json::Value = serde_json::from_str(&with_fields.out).unwrap();
    assert_eq!(
        document["message"],
        "--fields prints only the fields it names, so it cannot be used with --full"
    );
    assert_eq!(
        document["recovery"]["text"],
        "drop --full to print only the named fields, or drop --fields to print every field"
    );
    assert_eq!(
        document["fields"],
        serde_json::json!({"argument": "--full"})
    );
    assert_eq!(without_json.result.unwrap_err().number(), 2);
    assert_eq!(without_json.out, "");
    assert_eq!(
        without_json.err,
        "\
error: --full prints every field of every row in JSON, so it needs --json
  help:  add --json, or drop --full
  code:  tool::cli::usage
"
    );
}

#[test]
fn the_help_of_each_view_flag_states_what_the_command_prints_without_it() {
    use clap::Args as _;

    let command = crate::ViewFlags::augment_args(clap::Command::new("tool"));
    let help = |id: &str| {
        let arg = command.get_arguments().find(|arg| arg.get_id() == id);
        arg.and_then(clap::Arg::get_help).map(ToString::to_string)
    };

    assert_eq!(
        help("json").as_deref(),
        Some("Print the result as a JSON document. The default prints text.")
    );
    assert_eq!(
        help("fields").as_deref(),
        Some(
            "Print only these comma-separated fields, in this order. The default prints the compact columns."
        )
    );
    assert_eq!(
        help("long").as_deref(),
        Some(
            "Add the long columns after the compact ones. The default prints only the compact columns."
        )
    );
    assert_eq!(
        help("full").as_deref(),
        Some(
            "With --json, print every field of every row. The default prints only the fields of the columns."
        )
    );
}
