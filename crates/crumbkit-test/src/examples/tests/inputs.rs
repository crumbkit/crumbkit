//! Tests for the working directory and the standard input of each example
//! run.

use std::fs;
use std::time::Instant;

use crumbkit::Tool;
use crumbkit::help::{Example, Help, Leaf};
use snapbox::assert::Action;

use super::super::inputs::spellings;
use super::{FILES, TOOL, examples, run};
use crate::allow::Allow;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The tool record of one example that prints its standard input.
const CAT: Tool = Tool::new().help(
    Help::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["cat"], "Echo the input")])]),
);

#[test]
fn an_example_runs_in_the_directory_that_current_dir_names() {
    const LS: Tool = Tool::new()
        .help(Help::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["ls"], "List")])]));
    let dir = TempDir::new("examples-current-dir").unwrap();
    let work = TempDir::new("examples-current-dir-work").unwrap();
    work.write("marker", "");
    let path = work.path().display().to_string();

    run(&LS, &examples(&dir).current_dir(&path), Action::Overwrite);

    let output = fs::read_to_string(dir.path().join("sh-c-ls.txt")).unwrap();
    assert_eq!(output, "marker\n");
}

#[test]
fn a_relative_current_dir_resolves_from_the_test_s_directory() {
    const FIND: Tool =
        Tool::new().help(
            Help::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
                &["test -f lib.rs && echo found"],
                "Find a file",
            )])]),
        );
    let dir = TempDir::new("examples-relative-dir").unwrap();

    run(&FIND, &examples(&dir).current_dir("src"), Action::Overwrite);

    let output = fs::read_to_string(dir.path().join("sh-c-test-f-lib.rs-echo-found.txt")).unwrap();
    assert_eq!(output, "found\n");
}

#[test]
fn a_current_dir_that_is_not_a_directory_fails_the_check_once_naming_it() {
    let dir = TempDir::new("examples-missing-dir").unwrap();

    assert_eq!(
        panic_message(|| run(
            &TOOL,
            &examples(&dir).current_dir("/nonexistent/work"),
            Action::Overwrite
        ))
        .unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         Examples::current_dir names `/nonexistent/work`, which is not a directory, so the check runs no example"
    );
    assert!(dir.names().is_empty(), "{:?}", dir.names());
}

#[test]
fn an_example_reads_the_bytes_of_its_stdin_file_on_standard_input() {
    let dir = TempDir::new("examples-stdin").unwrap();
    dir.write("sh-c-cat.stdin", "first\nsecond\n");

    run(&CAT, &examples(&dir), Action::Overwrite);

    let output = fs::read_to_string(dir.path().join("sh-c-cat.txt")).unwrap();
    assert_eq!(output, "first\nsecond\n");
    run(&CAT, &examples(&dir), Action::Verify);
}

#[test]
fn an_example_without_a_stdin_file_reads_an_empty_standard_input() {
    const READS: Tool = Tool::new().help(
        Help::new()
            .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["cat; echo end"], "Read")])]),
    );
    let dir = TempDir::new("examples-no-stdin").unwrap();
    let started = Instant::now();

    run(&READS, &examples(&dir), Action::Overwrite);

    assert!(started.elapsed() < super::super::TIMEOUT);
    let output = fs::read_to_string(dir.path().join("sh-c-cat-echo-end.txt")).unwrap();
    assert_eq!(output, "end\n");
}

#[test]
fn a_stdin_file_that_no_example_names_fails_the_check_and_overwrite_keeps_it() {
    let dir = TempDir::new("examples-stale-stdin").unwrap();
    dir.write("sh-c-echo-titles.stdin", "titles\n");

    assert_eq!(
        panic_message(|| run(&TOOL, &examples(&dir), Action::Overwrite)).unwrap(),
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             the input file `{}` belongs to no example in the tool record, and the check never deletes it",
            dir.path().join("sh-c-echo-titles.stdin").display()
        )
    );
    assert!(dir.names().contains(&"sh-c-echo-titles.stdin".to_owned()));
}

#[test]
fn an_opted_out_example_keeps_its_stdin_file() {
    const ALLOW: &[Allow] = &[Allow::example(
        "-c",
        &["echo books"],
        "The output names the day.",
    )];
    let dir = TempDir::new("examples-opted-out-stdin").unwrap();
    dir.write("sh-c-echo-books.stdin", "books\n");

    run(&TOOL, &examples(&dir).allow(ALLOW), Action::Overwrite);

    assert_eq!(dir.names(), [FILES[0], "sh-c-echo-books.stdin", FILES[2]]);
}

#[test]
fn a_verbatim_drive_path_is_redacted_in_its_spelling_without_the_prefix_too() {
    let found = spellings(std::path::Path::new(
        r"\\?\C:\Users\run\Temp\crumbkit-test-1",
    ));

    assert_eq!(
        found,
        [
            r"\\?\C:\Users\run\Temp\crumbkit-test-1",
            r"C:\Users\run\Temp\crumbkit-test-1",
        ]
    );
}

#[test]
fn a_verbatim_unc_path_keeps_its_prefix_and_has_one_spelling() {
    let found = spellings(std::path::Path::new(r"\\?\UNC\server\share\dir"));

    assert_eq!(found, [r"\\?\UNC\server\share\dir"]);
}

#[test]
fn a_plain_path_has_one_spelling() {
    assert_eq!(spellings(std::path::Path::new("/tmp/work")), ["/tmp/work"]);
}
