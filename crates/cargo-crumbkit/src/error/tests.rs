//! Tests for the failures' messages and next steps.

use std::io;
use std::path::PathBuf;

use crumbkit::{Failure as _, Recovery};

use super::WeighError;

/// Returns the message and the hint of `error`.
fn rendered(error: &WeighError) -> (String, String) {
    let Recovery::Hint(hint) = error.recovery() else {
        panic!("{error} has a next step other than a hint");
    };
    (error.to_string(), hint)
}

/// Returns an I/O error of the kind `NotFound`.
fn not_found() -> io::Error {
    io::Error::from(io::ErrorKind::NotFound)
}

#[test]
fn every_failure_names_what_failed_and_hints_at_what_to_change() {
    let cases = [
        (
            WeighError::CargoUnavailable {
                cargo: "cargo".to_owned(),
                error: not_found(),
            },
            "cargo cannot start as `cargo`",
            "run cargo crumbkit through cargo, which sets CARGO to its own path",
        ),
        (
            WeighError::BuildFailed {
                bin: "shelf".to_owned(),
                status: "exit status: 101".to_owned(),
            },
            "`cargo build --workspace --bin shelf` exited with exit status: 101",
            "run `cargo build --workspace --bin shelf` to see cargo's errors",
        ),
        (
            WeighError::NoBinary {
                bin: "shelf".to_owned(),
            },
            "cargo built no executable for the binary shelf",
            "run `cargo build --workspace --bin shelf --message-format json` to see the artifacts that cargo built",
        ),
        (
            WeighError::NoTempDir { error: not_found() },
            "the temporary XDG directories for the tool's runs cannot be created",
            "make the system's temporary directory writable, or set TMPDIR to one that is",
        ),
        (
            WeighError::ToolUnavailable {
                binary: PathBuf::from("/work/target/debug/shelf"),
                error: not_found(),
            },
            "the binary /work/target/debug/shelf cannot start",
            "check that the binary runs on this machine",
        ),
        (
            WeighError::NoCommandsList {
                line: "shelf commands list --all --json --full".to_owned(),
            },
            "`shelf commands list --all --json --full` prints no crumbkit/commands-list/v1 document, so the tool's commands cannot be read",
            "add a `commands list` command at the tool's root that takes crumbkit::commands::CommandsList, as the crumbkit::commands module shows",
        ),
        (
            WeighError::HelpFailed {
                line: "shelf books -h".to_owned(),
                status: "exit status: 2".to_owned(),
            },
            "`shelf books -h` exited with exit status: 2",
            "run `shelf books -h` to see the failure",
        ),
        (
            WeighError::GoldenUnreadable {
                path: PathBuf::from("tests/golden/examples/shelf-books-list.txt"),
                error: not_found(),
            },
            "the golden file tests/golden/examples/shelf-books-list.txt cannot be read",
            "make the golden directory readable, or pass --goldens another directory",
        ),
        (
            WeighError::NoUsageFigures {
                line: "shelf self figures list --all --json --log usage.jsonl".to_owned(),
            },
            "`shelf self figures list --all --json --log usage.jsonl` prints no crumbkit/self-figures-list/v1 document, so the usage figures cannot be read",
            "run `shelf self figures list --all --json --log usage.jsonl` to see the failure; a tool needs a `self figures list` command that takes crumbkit::usage::SelfFigures, as shelf's shows",
        ),
        (
            WeighError::NoStateDir,
            "the usage log cannot be found, because the XDG state directory cannot be found",
            "set HOME or XDG_STATE_HOME, or pass --log the usage log",
        ),
    ];

    for (error, message, hint) in cases {
        assert_eq!(rendered(&error), (message.to_owned(), hint.to_owned()));
    }
}
