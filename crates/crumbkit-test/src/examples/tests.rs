//! Tests for the example check.
//!
//! The tests run `/bin/sh`, which Linux and macOS, crumbkit's supported
//! platforms, both have, so they need no binary of their own. The tree of
//! `sh()` names sh's options as commands, so each example's words are a
//! script that prints, fails or writes to standard error as a test needs.

use std::fs;
use std::path::Path;

use clap::Command;
use crumbkit::help::{Example, Guide, Leaf};
use snapbox::Assert;
use snapbox::assert::Action;

use super::{Examples, check, file_name};
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The binary that runs every example.
const SH: &str = "/bin/sh";

/// A tool whose commands are options of `/bin/sh`: `sh -c <script>` runs a
/// script, and `sh -e -c <script>` runs it and stops at the first command
/// that fails. `-e` is hidden.
fn sh() -> Command {
    Command::new("sh")
        .subcommand(Command::new("-c"))
        .subcommand(Command::new("-e").hide(true).subcommand(Command::new("-c")))
}

/// The guide of `sh()`, whose three examples each print one word, and whose
/// last leaf names no command of `sh()`.
const GUIDE: Guide = Guide::new().leaves(&[
    Leaf::new(&["-c"]).examples(&[
        Example::new(&["echo books"], "Print a word"),
        Example::new(&["echo authors"], "Print another word"),
    ]),
    Leaf::new(&["-e", "-c"]).examples(&[Example::new(&["echo shelves"], "Print a third word")]),
    Leaf::new(&["books"]).examples(&[Example::new(&[], "Run a script named books")]),
]);

/// The golden files of `GUIDE`, sorted by name.
const FILES: [&str; 3] = [
    "sh-c-echo-authors.txt",
    "sh-c-echo-books.txt",
    "sh-e-c-echo-shelves.txt",
];

/// Returns the settings that run `/bin/sh` and keep the golden files in
/// `dir`.
fn examples(dir: &TempDir) -> Examples<'_> {
    Examples::new(SH, dir.path().to_str().unwrap())
}

/// Runs the check of `guide` on `sh()` with `examples` and `action`.
fn run(guide: &Guide, examples: &Examples<'_>, action: Action) {
    check(&sh(), guide, examples, &Assert::new().action(action));
}

#[test]
fn overwrite_writes_each_example_s_standard_output_to_its_golden_file_and_deletes_a_stale_one() {
    let dir = TempDir::new("examples-overwrite").unwrap();
    dir.write("sh-c-echo-titles.txt", "titles\n");
    dir.write("notes.md", "Only .txt files are golden files.\n");

    run(&GUIDE, &examples(&dir), Action::Overwrite);

    let mut expected: Vec<&str> = FILES.to_vec();
    expected.insert(0, "notes.md");
    assert_eq!(dir.names(), expected);
    let output = fs::read_to_string(dir.path().join("sh-c-echo-books.txt")).unwrap();
    assert_eq!(output, "books\n");
}

#[test]
fn golden_files_that_hold_every_example_s_output_pass() {
    let dir = TempDir::new("examples-pass").unwrap();
    run(&GUIDE, &examples(&dir), Action::Overwrite);

    run(&GUIDE, &examples(&dir), Action::Verify);
}

#[test]
fn an_example_without_a_golden_file_fails_the_check() {
    let dir = TempDir::new("examples-missing").unwrap();
    let file = |name: &str| dir.path().join(name).display().to_string();

    assert_eq!(
        panic_message(|| run(&GUIDE, &examples(&dir), Action::Verify)).unwrap(),
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             sh -c 'echo books': the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             sh -c 'echo authors': the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             sh -e -c 'echo shelves': the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it",
            file("sh-c-echo-books.txt"),
            file("sh-c-echo-authors.txt"),
            file("sh-e-c-echo-shelves.txt"),
        )
    );
}

#[test]
fn a_golden_file_whose_content_differs_fails_the_check() {
    let dir = TempDir::new("examples-differs").unwrap();
    run(&GUIDE, &examples(&dir), Action::Overwrite);
    dir.write("sh-c-echo-books.txt", "volumes\n");

    let message = panic_message(|| run(&GUIDE, &examples(&dir), Action::Verify)).unwrap();

    let heading = format!(
        "the command tree breaks crumbkit's example rules:\n\
         sh -c 'echo books': the output differs from the golden file `{}`:\n",
        dir.path().join("sh-c-echo-books.txt").display()
    );
    assert!(message.starts_with(&heading), "{message}");
    assert!(message.contains("volumes"), "{message}");
    assert_eq!(
        message.matches(": the output differs").count(),
        1,
        "{message}"
    );
}

#[test]
fn a_stale_golden_file_fails_the_check() {
    let dir = TempDir::new("examples-stale").unwrap();
    run(&GUIDE, &examples(&dir), Action::Overwrite);
    dir.write("sh-c-echo-titles.txt", "titles\n");

    assert_eq!(
        panic_message(|| run(&GUIDE, &examples(&dir), Action::Verify)).unwrap(),
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             the golden file `{}` belongs to no checked example, and `SNAPSHOTS=overwrite` deletes it",
            dir.path().join("sh-c-echo-titles.txt").display()
        )
    );
}

#[test]
fn an_example_that_exits_with_a_non_zero_code_fails_the_check_and_overwrite_writes_no_file() {
    const FAILS: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo partial; exit 3"], "Fail")])]);
    let dir = TempDir::new("examples-exit-code").unwrap();

    assert_eq!(
        panic_message(|| run(&FAILS, &examples(&dir), Action::Overwrite)).unwrap(),
        "these runs of the tool's examples fail:\n\
         sh -c 'echo partial; exit 3': an example must exit with code 0 and write nothing to standard error, and the run ends with exit status: 3"
    );
    assert!(dir.names().is_empty(), "{:?}", dir.names());
}

#[test]
fn an_example_that_writes_to_standard_error_fails_the_check() {
    const WARNS: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo warning >&2"], "Warn")])]);
    let dir = TempDir::new("examples-stderr").unwrap();

    assert_eq!(
        panic_message(|| run(&WARNS, &examples(&dir), Action::Verify)).unwrap(),
        "these runs of the tool's examples fail:\n\
         sh -c 'echo warning >&2': an example must exit with code 0 and write nothing to standard error, and the run ends with exit status: 0 after writing this to standard error:\n\
         warning\n"
    );
}

#[test]
fn a_binary_that_cannot_start_fails_the_check_naming_its_path_and_the_error() {
    const ONE: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["echo books"], "Print")])]);
    const MISSING: Examples<'_> = Examples::new("/nonexistent/sh", "tests/golden/examples");

    let message = panic_message(|| check(&sh(), &ONE, &MISSING, &Assert::new())).unwrap();

    assert!(
        message.starts_with(
            "these runs of the tool's examples fail:\n\
             sh -c 'echo books': the binary `/nonexistent/sh` cannot start: "
        ),
        "{message}"
    );
    assert_eq!(message.lines().count(), 2, "{message}");
}

#[test]
fn failed_runs_and_broken_rules_are_reported_under_their_own_headings() {
    const FAILS: Guide =
        Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["exit 3"], "Fail")])]);
    let dir = TempDir::new("examples-headings").unwrap();

    assert_eq!(
        panic_message(|| run(
            &FAILS,
            &examples(&dir).exempt(&["books"]),
            Action::Overwrite
        ))
        .unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         the exemption `books` names no command\n\
         these runs of the tool's examples fail:\n\
         sh -c 'exit 3': an example must exit with code 0 and write nothing to standard error, and the run ends with exit status: 3"
    );
}

#[test]
fn the_examples_of_an_exempt_command_and_everything_below_it_are_skipped() {
    let dir = TempDir::new("examples-exempt").unwrap();
    run(&GUIDE, &examples(&dir), Action::Overwrite);

    run(&GUIDE, &examples(&dir).exempt(&["-e"]), Action::Overwrite);

    assert_eq!(dir.names(), FILES[..2]);
}

#[test]
fn an_exemption_that_names_no_command_fails_the_check() {
    let dir = TempDir::new("examples-exemption").unwrap();

    assert_eq!(
        panic_message(|| run(
            &GUIDE,
            &examples(&dir).exempt(&["books"]),
            Action::Overwrite
        ))
        .unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         the exemption `books` names no command"
    );
}

#[test]
fn each_run_has_a_cleared_environment_with_the_tool_s_variables_and_fresh_xdg_directories() {
    const ENVIRONMENT: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(
        &[r#"echo "[$HOME] [$SHELF_DATA]"; for dir in "$XDG_CONFIG_HOME" "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"; do test -d "$dir" && echo "$dir"; done"#],
        "Print the environment",
    )])]);
    let dir = TempDir::new("examples-environment").unwrap();
    let examples = examples(&dir).env(&[("SHELF_DATA", "tests/fixtures/books.json")]);

    run(&ENVIRONMENT, &examples, Action::Overwrite);

    let names = dir.names();
    let output = fs::read_to_string(dir.path().join(&names[0])).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 5, "{output}");
    assert_eq!(lines[0], "[] [tests/fixtures/books.json]");
    for (line, name) in lines[1..].iter().zip(["config", "data", "state", "cache"]) {
        let path = Path::new(line);
        assert!(path.ends_with(name), "{output}");
        assert!(!path.exists(), "the check removes {line} after the run");
    }
}

#[test]
fn a_golden_file_is_named_after_the_tool_the_path_and_the_words_joined_by_dashes() {
    assert_eq!(
        file_name("shelf", &["books", "list"], &[]),
        "shelf-books-list.txt"
    );
    assert_eq!(
        file_name("shelf", &["books", "list"], &["--all"]),
        "shelf-books-list-all.txt"
    );
    assert_eq!(
        file_name("shelf", &["books", "show"], &["2", "5"]),
        "shelf-books-show-2-5.txt"
    );
    assert_eq!(
        file_name(
            "shelf",
            &["books", "list"],
            &["--fields", "title, year", "--", "v1.2_rc/"]
        ),
        "shelf-books-list-fields-title-year-v1.2_rc.txt"
    );
}

#[test]
fn two_examples_with_one_golden_file_name_fail_the_check_naming_both_command_lines() {
    const TWINS: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[
        Example::new(&["echo books"], "Print a word"),
        Example::new(&["echo 'books'"], "Print the same word"),
    ])]);
    let dir = TempDir::new("examples-collision").unwrap();

    assert_eq!(
        panic_message(|| run(&TWINS, &examples(&dir), Action::Overwrite)).unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         sh -c 'echo '\\''books'\\''': the example's golden file name `sh-c-echo-books.txt` is also the name for the example `sh -c 'echo books'`"
    );
}
