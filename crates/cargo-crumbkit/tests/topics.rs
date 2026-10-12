//! End-to-end tests of the help topics: each command line that names a
//! topic runs, as a caller types it, through the built binary.

use std::process::Command;

/// Runs `line`, a command line that starts with `cargo crumbkit`, as cargo
/// runs it: cargo finds the binary `cargo-crumbkit` and passes it every
/// word after `cargo`. Returns the run's standard output after checking
/// that the run succeeded, or `None` when the binary cannot start.
fn run_as_cargo_does(line: &str) -> Option<String> {
    let mut words = line.split(' ');
    assert_eq!(words.next(), Some("cargo"), "{line}");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(words)
        .output()
        .ok()?;
    assert!(output.status.success(), "{line}: {output:?}");
    String::from_utf8(output.stdout).ok()
}

/// Returns the command in the first pair of backticks of the line of
/// `text` that starts with `start`, or `None` when no line has one.
fn quoted(text: &str, start: &str) -> Option<String> {
    let line = text.lines().find(|line| line.starts_with(start))?;
    line.split('`').nth(1).map(str::to_owned)
}

#[test]
fn the_topics_line_of_cargo_crumbkit_s_help_names_a_command_that_prints_the_topic() {
    let help = run_as_cargo_does("cargo crumbkit -h").unwrap();
    let line = quoted(&help, "Topics:")
        .unwrap()
        .replace("<TOPIC>", "exit-codes");

    assert_eq!(line, "cargo crumbkit help exit-codes");
    assert!(
        run_as_cargo_does(&line)
            .unwrap()
            .starts_with("Exit codes\n")
    );
}

#[test]
fn the_exit_code_note_of_commands_list_names_a_command_that_prints_the_topic() {
    let list = run_as_cargo_does("cargo crumbkit commands list").unwrap();
    let line = quoted(&list, "Exit codes:").unwrap();

    assert_eq!(line, "cargo crumbkit help exit-codes");
    assert!(
        run_as_cargo_does(&line)
            .unwrap()
            .starts_with("Exit codes\n")
    );
}

#[test]
fn the_topic_list_ends_with_a_command_that_prints_a_topic() {
    let topics = run_as_cargo_does("cargo crumbkit help").unwrap();
    let next = topics
        .lines()
        .last()
        .unwrap()
        .strip_prefix("Next: ")
        .unwrap()
        .replace("<TOPIC>", "exit-codes");

    assert_eq!(next, "cargo crumbkit help exit-codes");
    assert!(
        run_as_cargo_does(&next)
            .unwrap()
            .starts_with("Exit codes\n")
    );
}
