//! Tests for running the tool's binary.
//!
//! Most tests run programs that Linux and macOS both have: `/bin/sh`,
//! `/bin/echo` and `/usr/bin/env`. Windows lacks them, so those tests run
//! only on Unix.

use std::path::Path;

use crumbkit::Spelling;

use super::{Listed, Tool, Used, listed, used};

/// The start of a `crumbkit/commands-list/v1` document of two commands, with
/// fields that `weigh` does not read.
const DOCUMENT: &str = r#"{"schema":"crumbkit/commands-list/v1","data":[
{"path":"shelf","about":"Browse","long_about":null,"arguments":[],"examples":[],"columns":[]},
{"path":"shelf books list","about":"List","long_about":null,"arguments":[],"examples":[
{"command":"shelf books list","about":"Print the first page"},
{"command":"shelf books list --author 'Jane Austen'","about":"Print one author's books"}],"columns":[]}
],"total":2}"#;

/// Returns the spelled code of `error`.
fn code(error: &impl crumbkit::Failure) -> String {
    Spelling::COLONS.spell(error.code())
}

#[test]
fn a_commands_document_gives_each_command_s_path_and_example_command_lines() {
    assert_eq!(
        listed(DOCUMENT.as_bytes()),
        Some(vec![
            Listed {
                path: "shelf".to_owned(),
                examples: vec![],
            },
            Listed {
                path: "shelf books list".to_owned(),
                examples: vec![
                    "shelf books list".to_owned(),
                    "shelf books list --author 'Jane Austen'".to_owned(),
                ],
            },
        ])
    );
}

#[test]
fn output_that_is_not_a_commands_document_gives_none() {
    let other = DOCUMENT.replace("crumbkit/commands-list/v1", "shelf/books-list/v1");

    assert_eq!(listed(other.as_bytes()), None);
    assert_eq!(listed(b"PATH  ABOUT\nshelf  Browse\n"), None);
}

/// A `crumbkit/self-figures-list/v1` document of two commands, with fields
/// that `weigh` does not read.
const FIGURES: &str = r#"{"schema":"crumbkit/self-figures-list/v1","data":[
{"command":"shelf","runs":0,"help_pages":1,"help_before":0,"refused":0,"failed":0,"p95_ms":null,"p95_out_bytes":null,"typed_full":0,"typed_all":0,"typed_long":0},
{"command":"books show","runs":5,"help_pages":0,"help_before":3,"refused":0,"failed":2,"p95_ms":2,"p95_out_bytes":86,"typed_full":0,"typed_all":0,"typed_long":0}
],"total":2,"summary":{"runs":6,"sessions":1,"callers":[],"since":"2026-10-04","skipped":0}}"#;

#[test]
fn a_figures_document_gives_each_command_s_runs_help_before_and_p95_output() {
    assert_eq!(
        used(FIGURES.as_bytes()),
        Some(vec![
            Used {
                command: "shelf".to_owned(),
                runs: 0,
                help_before: 0,
                p95_out_bytes: None,
            },
            Used {
                command: "books show".to_owned(),
                runs: 5,
                help_before: 3,
                p95_out_bytes: Some(86),
            },
        ])
    );
}

#[test]
fn output_that_is_not_a_figures_document_gives_none() {
    let other = FIGURES.replace("crumbkit/self-figures-list/v1", "crumbkit/self-usage/v1");

    assert_eq!(used(other.as_bytes()), None);
    assert_eq!(used(DOCUMENT.as_bytes()), None);
}

// The test runs a program that Windows lacks.
#[cfg(unix)]
#[test]
fn a_tool_without_self_figures_list_fails_with_no_usage_figures() {
    let tool = Tool::new(Path::new("/bin/sh"), &std::env::temp_dir()).unwrap();

    // `sh self figures list ...` cannot open a script named `self`.
    let error = tool
        .figures(Path::new("/work/usage.jsonl"), Some("1d"))
        .unwrap_err();

    assert_eq!(code(&error), "crumbkit::weigh::no_usage_figures");
    assert_eq!(
        error.to_string(),
        "`sh self figures list --all --json --log /work/usage.jsonl --since 1d` prints no crumbkit/self-figures-list/v1 document, so the usage figures cannot be read"
    );
}

// The test runs a program that Windows lacks.
#[cfg(unix)]
#[test]
fn each_run_has_only_the_xdg_variables_each_pointing_at_a_directory_that_exists() {
    let tool = Tool::new(Path::new("/usr/bin/env"), &std::env::temp_dir()).unwrap();

    let output = tool.run(&[]).unwrap();

    let text = String::from_utf8(output.stdout).unwrap();
    let mut names = Vec::new();
    for line in text.lines() {
        let (name, value) = line.split_once('=').unwrap();
        assert!(Path::new(value).is_dir(), "{line}");
        assert!(
            value.starts_with(tool.xdg.path().to_str().unwrap()),
            "{line}"
        );
        names.push(name);
    }
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "XDG_CACHE_HOME",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_STATE_HOME"
        ]
    );
}

// The test runs a program that Windows lacks.
#[cfg(unix)]
#[test]
fn a_help_page_weighs_the_bytes_the_run_writes_to_standard_output() {
    let tool = Tool::new(Path::new("/bin/echo"), &std::env::temp_dir()).unwrap();

    // `/bin/echo books -h` prints `books -h` and a newline.
    assert_eq!(tool.help_bytes("echo books", "-h").unwrap(), 9);
}

// The test runs a program that Windows lacks.
#[cfg(unix)]
#[test]
fn a_help_run_that_exits_with_a_failure_fails_with_help_failed() {
    let tool = Tool::new(Path::new("/bin/sh"), &std::env::temp_dir()).unwrap();

    // `sh no-such-script --help` cannot open the script and exits with 127.
    let error = tool.help_bytes("sh no-such-script", "--help").unwrap_err();

    assert_eq!(code(&error), "crumbkit::weigh::help_failed");
    assert!(
        error
            .to_string()
            .starts_with("`sh no-such-script --help` exited with "),
        "{error}"
    );
}

// The test runs a program that Windows lacks.
#[cfg(unix)]
#[test]
fn a_tool_without_commands_list_fails_with_no_commands_list() {
    let tool = Tool::new(Path::new("/bin/sh"), &std::env::temp_dir()).unwrap();

    // `sh commands list ...` cannot open a script named `commands`.
    let error = tool.commands().unwrap_err();

    assert_eq!(code(&error), "crumbkit::weigh::no_commands_list");
    assert_eq!(
        error.to_string(),
        "`sh commands list --all --json --full` prints no crumbkit/commands-list/v1 document, so the tool's commands cannot be read"
    );
}

#[test]
fn xdg_directories_that_cannot_be_created_fail_with_no_temp_dir() {
    let error = Tool::new(Path::new("/bin/sh"), Path::new("/nonexistent/tmp"))
        .err()
        .unwrap();

    assert_eq!(code(&error), "crumbkit::weigh::no_temp_dir");
}

#[test]
fn a_binary_that_cannot_start_fails_with_tool_unavailable() {
    let tool = Tool::new(Path::new("/nonexistent/shelf"), &std::env::temp_dir()).unwrap();

    let error = tool.help_bytes("shelf", "-h").unwrap_err();

    assert_eq!(code(&error), "crumbkit::weigh::tool_unavailable");
}
