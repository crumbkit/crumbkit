//! Tests for the view flags that `finish_to` refuses because the result
//! cannot use them, for a tool that parses without crumbkit's `parse`.

use std::process::ExitCode;

use schemars::JsonSchema;
use serde::Serialize;

use super::finish_with;
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
  code:  crumbkit::cli::usage
"
    );
}

#[test]
fn finish_refuses_long_and_full_for_a_prose_result() {
    let eight = || Prose::new("Eight planets.".to_owned());

    let long = finish_with(&["--long"], COUNT, eight());
    let full = finish_with(&["--json", "--full"], COUNT, eight());

    assert_eq!(long.code, ExitCode::from(2));
    assert!(
        long.err.starts_with(
            "error: --long adds the long columns of a table, and this command prints prose\n"
        ),
        "{}",
        long.err
    );
    assert_eq!(full.code, ExitCode::from(2));
    assert!(
        full.out.contains(
            r#""message":"--full prints every field of every row, and this command prints prose""#
        ),
        "{}",
        full.out
    );
}
