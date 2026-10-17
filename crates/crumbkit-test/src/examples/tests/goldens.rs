//! Tests for how the example check compares an output with its golden
//! file: the check's own XDG paths print as their variable names, and the
//! golden file's text is compared literally.

use std::fs;

use crumbkit::help::{Example, Guide, Leaf};
use snapbox::assert::Action;

use super::{examples, run};
use crate::temp::TempDir;
use crate::testing::panic_message;

#[test]
fn the_check_s_own_xdg_directories_are_redacted_by_their_variable_names() {
    const XDG: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &[r#"echo "$XDG_CONFIG_HOME $XDG_DATA_HOME $XDG_STATE_HOME $XDG_CACHE_HOME""#],
        "Print the XDG directories",
    )])]);
    let dir = TempDir::new("examples-goldens-xdg").unwrap();

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
