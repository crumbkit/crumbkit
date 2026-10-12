//! Tests for the values that the example check redacts before it compares
//! an output with its golden file.

use std::fs;

use crumbkit::help::{Example, Guide, Leaf};
use snapbox::assert::Action;

use super::{examples, run};
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The guide of one example that prints a path below `$ROOT` on standard
/// output and `$ROOT` on standard error.
const ROOTED: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
    &[r#"echo "$ROOT/books"; echo "$ROOT" >&2"#],
    "Print a path",
)])]);

/// The golden files of `ROOTED`'s example.
const ROOTED_OUT: &str = "sh-c-echo-ROOT-books-echo-ROOT-2.txt";
const ROOTED_ERR: &str = "sh-c-echo-ROOT-books-echo-ROOT-2.stderr";

#[test]
fn a_redacted_value_matches_its_placeholder_in_the_golden_file() {
    let dir = TempDir::new("examples-redact-match").unwrap();
    let root = TempDir::new("examples-redact-match-root").unwrap();
    let path = root.path().display().to_string();
    dir.write(ROOTED_OUT, "[ROOT]/books\n");
    dir.write(ROOTED_ERR, "[ROOT]\n");
    let env = [("ROOT", path.as_str())];
    let redact = [("[ROOT]", path.as_str())];

    run(
        &ROOTED,
        &examples(&dir).env(&env).redact(&redact),
        Action::Verify,
    );
}

#[test]
fn overwrite_writes_the_placeholder_into_the_golden_file() {
    let dir = TempDir::new("examples-redact-overwrite").unwrap();
    let root = TempDir::new("examples-redact-overwrite-root").unwrap();
    let path = root.path().display().to_string();
    let env = [("ROOT", path.as_str())];
    let redact = [("[ROOT]", path.as_str())];

    run(
        &ROOTED,
        &examples(&dir).env(&env).redact(&redact),
        Action::Overwrite,
    );

    let output = fs::read_to_string(dir.path().join(ROOTED_OUT)).unwrap();
    assert_eq!(output, "[ROOT]/books\n");
}

#[test]
fn a_redacted_value_in_standard_error_matches_its_placeholder() {
    let dir = TempDir::new("examples-redact-stderr").unwrap();
    let root = TempDir::new("examples-redact-stderr-root").unwrap();
    let path = root.path().display().to_string();
    let env = [("ROOT", path.as_str())];
    let redact = [("[ROOT]", path.as_str())];

    run(
        &ROOTED,
        &examples(&dir).env(&env).redact(&redact),
        Action::Overwrite,
    );

    let note = fs::read_to_string(dir.path().join(ROOTED_ERR)).unwrap();
    assert_eq!(note, "[ROOT]\n");
}

#[test]
fn a_placeholder_that_snapbox_rejects_or_an_empty_value_fails_the_check() {
    const ONE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo books"], "Print")])]);
    let dir = TempDir::new("examples-redact-invalid").unwrap();
    let redact = [("[root]", "/tmp"), ("[]", "/tmp"), ("[ROOT]", "")];

    assert_eq!(
        panic_message(|| run(&ONE, &examples(&dir).redact(&redact), Action::Overwrite)).unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         Examples::redact names the placeholder `[root]`, and a placeholder is `[` and `]` around one or more capital letters A to Z and underscores\n\
         Examples::redact names the placeholder `[]`, and a placeholder is `[` and `]` around one or more capital letters A to Z and underscores\n\
         Examples::redact gives the placeholder `[ROOT]` an empty value, which redacts nothing"
    );
}

#[test]
fn the_check_s_own_xdg_directories_are_redacted_by_their_variable_names() {
    const XDG: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &[r#"echo "$XDG_CONFIG_HOME $XDG_DATA_HOME $XDG_STATE_HOME $XDG_CACHE_HOME""#],
        "Print the XDG directories",
    )])]);
    let dir = TempDir::new("examples-redact-xdg").unwrap();

    run(&XDG, &examples(&dir), Action::Overwrite);

    let names = dir.names();
    let output = fs::read_to_string(dir.path().join(&names[0])).unwrap();
    assert_eq!(
        output,
        "[XDG_CONFIG_HOME] [XDG_DATA_HOME] [XDG_STATE_HOME] [XDG_CACHE_HOME]\n"
    );
    run(&XDG, &examples(&dir), Action::Verify);
}

#[test]
fn a_wildcard_in_an_example_golden_file_is_literal_text() {
    const LITERAL: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo literal"], "Print")])]);
    let dir = TempDir::new("examples-wildcard").unwrap();
    dir.write("sh-c-echo-literal.txt", "lit[..]\n");

    let message = panic_message(|| run(&LITERAL, &examples(&dir), Action::Verify)).unwrap();

    assert!(
        message.contains("sh -c 'echo literal': the output differs from the golden file"),
        "{message}"
    );
}
