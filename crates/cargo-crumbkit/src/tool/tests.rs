//! Tests for running the tool's binary.
//!
//! The tests run programs that Linux and macOS, crumbkit's supported
//! platforms, both have: `/bin/sh`, `/bin/echo` and `/usr/bin/env`.

use std::path::Path;

use crumbkit::Spelling;

use super::{Listed, Tool, listed};

/// The start of a `crumbkit/commands/v1` document of two commands, with
/// fields that `weigh` does not read.
const DOCUMENT: &str = r#"{"schema":"crumbkit/commands/v1","data":[
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
    let other = DOCUMENT.replace("crumbkit/commands/v1", "shelf/books-list/v1");

    assert_eq!(listed(other.as_bytes()), None);
    assert_eq!(listed(b"PATH  ABOUT\nshelf  Browse\n"), None);
}

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

#[test]
fn a_help_page_weighs_the_bytes_the_run_writes_to_standard_output() {
    let tool = Tool::new(Path::new("/bin/echo"), &std::env::temp_dir()).unwrap();

    // `/bin/echo books -h` prints `books -h` and a newline.
    assert_eq!(tool.help_bytes("echo books", "-h").unwrap(), 9);
}

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

#[test]
fn a_tool_without_commands_list_fails_with_no_commands_list() {
    let tool = Tool::new(Path::new("/bin/sh"), &std::env::temp_dir()).unwrap();

    // `sh commands list ...` cannot open a script named `commands`.
    let error = tool.commands().unwrap_err();

    assert_eq!(code(&error), "crumbkit::weigh::no_commands_list");
    assert_eq!(
        error.to_string(),
        "`sh commands list --all --json --full` prints no crumbkit/commands/v1 document, so the tool's commands cannot be read"
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
