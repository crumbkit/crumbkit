//! Tests for the working directory and the standard input of each example
//! run.

use std::fs;
use std::time::Instant;

use crumbkit::help::{Example, Guide, Leaf};
use snapbox::assert::Action;

use super::{FILES, GUIDE, examples, run};
use crate::allow::Allow;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The guide of one example that prints its standard input.
const CAT: Guide = Guide::new()
    .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["cat"], "Echo the input")])]);

#[test]
fn an_example_runs_in_the_directory_that_current_dir_names() {
    const LS: Guide =
        Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["ls"], "List")])]);
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
    const FIND: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &["test -f lib.rs && echo found"],
        "Find a file",
    )])]);
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
            &GUIDE,
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
    const READS: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["cat; echo end"], "Read")])]);
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
        panic_message(|| run(&GUIDE, &examples(&dir), Action::Overwrite)).unwrap(),
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             the input file `{}` belongs to no example in the guide, and the check never deletes it",
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

    run(&GUIDE, &examples(&dir).allow(ALLOW), Action::Overwrite);

    assert_eq!(dir.names(), [FILES[0], "sh-c-echo-books.stdin", FILES[2]]);
}
