//! Tests for the failures' messages and next steps.

use std::io;
use std::path::PathBuf;

use crumbkit::{Failure as _, Recovery};

use crumbkit_markdown::render;

use super::{DocsError, SkillError, ToolError, WeighError};

/// Returns the message and the hint of `error`.
fn rendered(error: &impl crumbkit::Failure) -> (String, String) {
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
fn every_failure_of_building_and_running_the_tool_names_what_failed_and_hints_at_what_to_change() {
    let cases = [
        (
            ToolError::CargoUnavailable {
                cargo: "cargo".to_owned(),
                error: not_found(),
            },
            "cargo cannot start as `cargo`",
            "run cargo crumbkit through cargo, which sets CARGO to its own path",
        ),
        (
            ToolError::BuildFailed {
                bin: "shelf".to_owned(),
                status: "exit status: 101".to_owned(),
            },
            "`cargo build --workspace --bin shelf` exited with exit status: 101",
            "run `cargo build --workspace --bin shelf` to see cargo's errors",
        ),
        (
            ToolError::NoBinary {
                bin: "shelf".to_owned(),
            },
            "cargo built no executable for the binary shelf",
            "run `cargo build --workspace --bin shelf --message-format json` to see the artifacts that cargo built",
        ),
        (
            ToolError::NoTempDir { error: not_found() },
            "the temporary XDG directories for the tool's runs cannot be created",
            "make the system's temporary directory writable, or set TMPDIR to one that is",
        ),
        (
            ToolError::ToolUnavailable {
                binary: PathBuf::from("/work/target/debug/shelf"),
                error: not_found(),
            },
            "the binary /work/target/debug/shelf cannot start",
            "check that the binary runs on this machine",
        ),
        (
            ToolError::NoCommandsList {
                line: "shelf commands list --all --json --full".to_owned(),
            },
            "`shelf commands list --all --json --full` prints no commands-list/v1 document, so the tool's commands cannot be read",
            "add a `commands list` command at the tool's root that takes crumbkit::commands::CommandsList, as the crumbkit::commands module shows",
        ),
    ];

    for (error, message, hint) in cases {
        assert_eq!(rendered(&error), (message.to_owned(), hint.to_owned()));
    }
}

#[test]
fn every_failure_of_weigh_names_what_failed_and_hints_at_what_to_change() {
    let cases = [
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
            "`shelf self figures list --all --json --log usage.jsonl` prints no self-figures-list/v1 document, so the usage figures cannot be read",
            "run `shelf self figures list --all --json --log usage.jsonl` to see the failure; a tool needs a `self figures list` command that takes crumbkit_usage::SelfFigures, as shelf's shows",
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

#[test]
fn every_failure_of_docs_names_what_failed_and_hints_at_what_to_change() {
    let cases = [
        (
            DocsError::Unrenderable {
                line: "shelf commands list --all --json --full".to_owned(),
                error: render(b"{}").unwrap_err(),
            },
            "the reference cannot be written from the document that `shelf commands list --all --json --full` prints",
            "run `shelf commands list --all --json --full` to see the document, and change the tool's help as the cause says",
        ),
        (
            DocsError::WriteFailed {
                path: PathBuf::from("examples/shelf/docs/commands.md"),
                error: not_found(),
            },
            "the reference cannot be written to examples/shelf/docs/commands.md",
            "make the file's directory writable, or pass --out another file",
        ),
    ];

    for (error, message, hint) in cases {
        assert_eq!(rendered(&error), (message.to_owned(), hint.to_owned()));
    }
}

#[test]
fn a_skill_file_that_cannot_be_read_names_the_file_and_hints_at_another() {
    let error = SkillError::Unreadable {
        path: PathBuf::from("skill/SKILL.md"),
        error: not_found(),
    };

    assert_eq!(
        crumbkit::Spelling::COLONS.spell(error.code()),
        "crumbkit::skill::unreadable"
    );
    assert_eq!(
        rendered(&error),
        (
            "the skill file skill/SKILL.md cannot be read".to_owned(),
            "make the file readable, or pass --skill another file".to_owned()
        )
    );
}

#[test]
fn every_failure_of_skill_draft_names_its_code_what_failed_and_what_to_change() {
    let cases = [
        (
            SkillError::Unclosed {
                path: PathBuf::from("skill/SKILL.md"),
                section: "jobs",
                line: 12,
            },
            "crumbkit::skill::unclosed_section",
            "the jobs section that starts on line 12 of skill/SKILL.md has no end marker",
            "add the line <!-- /crumbkit:generated jobs --> where the section ends",
        ),
        (
            SkillError::WriteFailed {
                path: PathBuf::from("skill/SKILL.md"),
                error: not_found(),
            },
            "crumbkit::skill::write_failed",
            "the skill file skill/SKILL.md cannot be written",
            "make the file's directory writable, or pass --skill another file",
        ),
        (
            SkillError::UsageUnreadable {
                path: PathBuf::from("usage/a.jsonl"),
                error: not_found(),
            },
            "crumbkit::usage::unreadable",
            "the usage at usage/a.jsonl cannot be read",
            "make the path readable, or pass --usage another directory",
        ),
    ];

    for (error, code, message, hint) in cases {
        assert_eq!(crumbkit::Spelling::COLONS.spell(error.code()), code);
        assert_eq!(rendered(&error), (message.to_owned(), hint.to_owned()));
    }
}

#[test]
fn a_failure_to_build_or_run_the_tool_keeps_its_code_and_message_in_weigh_docs_and_skill_check() {
    let tool = || ToolError::NoBinary {
        bin: "shelf".to_owned(),
    };
    let weigh = WeighError::from(tool());
    let docs = DocsError::from(tool());
    let skill = SkillError::from(tool());

    for (code, message) in [
        (weigh.code(), weigh.to_string()),
        (docs.code(), docs.to_string()),
        (skill.code(), skill.to_string()),
    ] {
        assert_eq!(code, tool().code());
        assert_eq!(message, "cargo built no executable for the binary shelf");
    }
    assert_eq!(rendered(&weigh), rendered(&tool()));
    assert_eq!(rendered(&docs), rendered(&tool()));
    assert_eq!(rendered(&skill), rendered(&tool()));
}
