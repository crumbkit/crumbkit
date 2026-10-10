//! End-to-end tests of shelf's usage log. Each test runs the built binary
//! with a state directory of its own and reads the lines that the runs
//! appended to `shelf/usage.jsonl` there (ADR-0029).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::CommandFactory as _;
use serde_json::{Value, json};
use shelf::Cli;
use snapbox::cmd::{Command, OutputAssert};

/// Returns an empty directory for the test named `test`, with the
/// directories `config` and `state` under it still to be created by shelf.
///
/// # Errors
///
/// Returns the I/O error of a directory left by an earlier run that cannot
/// be removed.
fn home(test: &str) -> io::Result<PathBuf> {
    let home = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    #[expect(
        clippy::disallowed_methods,
        reason = "this helper owns the tests' file removals, which empty the directory of an earlier run"
    )]
    match fs::remove_dir_all(&home) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    Ok(home)
}

/// Runs the built `shelf` binary with `args`, a cleared environment and XDG
/// directories under `home`, and returns its output for assertions.
fn shelf(home: &Path, args: &[&str]) -> OutputAssert {
    Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .env_clear()
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .assert()
}

/// Returns each line of shelf's usage log under `home`, as JSON.
///
/// # Errors
///
/// Returns the error of a log that cannot be read or a line that is not
/// JSON.
fn lines(home: &Path) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let log = fs::read_to_string(home.join("state").join("shelf").join("usage.jsonl"))?;
    log.lines()
        .map(|line| Ok(serde_json::from_str(line)?))
        .collect()
}

/// Returns `line` after checking that the fields that change from run to
/// run have their types, and setting each to `"<checked>"`, so a test
/// compares the rest of the line exactly.
fn stable(mut line: Value) -> Value {
    let hash = |value: &Value| {
        value.as_str().is_some_and(|text| {
            text.len() == 16
                && text
                    .bytes()
                    .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
        })
    };
    assert!(
        line["at"].as_str().is_some_and(|at| at.ends_with('Z')),
        "{line}"
    );
    assert!(line["ms"].is_u64(), "{line}");
    assert!(hash(&line["session"]), "{line}");
    assert!(hash(&line["args"]), "{line}");
    for field in ["at", "ms", "session", "args"] {
        line[field] = json!("<checked>");
    }
    line
}

#[test]
fn a_run_appends_one_line_with_its_command_flags_exit_code_and_output_size() {
    let home = home("usage-run").unwrap();

    let run = shelf(&home, &["books", "list", "--limit", "2", "--json"]).success();

    let out_bytes = run.get_output().stdout.len();
    let lines = lines(&home).unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(
        stable(lines[0].clone()),
        json!({
            "schema": "crumbkit/usage/v1",
            "tool": "shelf",
            "version": Cli::command().get_version(),
            "at": "<checked>",
            "ms": "<checked>",
            "caller": "unknown",
            "session": "<checked>",
            "args": "<checked>",
            "command": "books list",
            "flags": ["--json", "--limit"],
            "exit": 0,
            "out_bytes": out_bytes,
            "truncated": true,
        })
    );
}

#[test]
fn a_refused_run_records_how_many_flags_were_unknown_and_the_nearest_real_one() {
    let home = home("usage-refused").unwrap();

    shelf(&home, &["books", "list", "--al"]).code(2);

    let line = stable(lines(&home).unwrap().remove(0));
    assert_eq!(line["command"], "books list");
    assert_eq!(line["unknown_flags"], 1);
    assert_eq!(line["nearest"], "--all");
    assert_eq!(line["exit"], 2);
    assert_eq!(line["code"], "crumbkit::cli::usage");
    assert_eq!(line.get("flags"), None);
}

#[test]
fn a_request_for_help_records_the_command_and_which_page() {
    let home = home("usage-help").unwrap();

    shelf(&home, &["help", "books", "show"]).success();
    shelf(&home, &["books", "-h"]).success();

    let lines = lines(&home).unwrap();
    assert_eq!(lines[0]["help"], "long");
    assert_eq!(lines[0]["command"], "books show");
    assert_eq!(lines[1]["help"], "short");
    assert_eq!(lines[1]["command"], "books");
    assert_eq!(lines[1]["exit"], 0);
}

#[test]
fn a_run_of_an_alias_records_the_alias_and_the_command_it_expanded_to() {
    let home = home("usage-alias").unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "this test owns its file writes, which set up the aliases file"
    )]
    {
        fs::create_dir_all(home.join("config").join("shelf")).unwrap();
        fs::write(
            home.join("config").join("shelf").join("aliases.toml"),
            "firsts = [\"books\", \"list\", \"--long\"]\n",
        )
        .unwrap();
    }

    shelf(&home, &["firsts", "--limit", "1"]).success();

    let line = stable(lines(&home).unwrap().remove(0));
    assert_eq!(line["alias"], true);
    assert_eq!(line["command"], "books list");
    assert_eq!(line["flags"], json!(["--limit", "--long"]));
}

#[test]
fn no_line_holds_a_value_that_the_caller_typed() {
    let home = home("usage-planted").unwrap();
    let runs: [&[&str]; 11] = [
        &["books", "list", "--fields", "s3cr3t-fields-PLANTED"],
        &["books", "list", "--fields=s3cr3t-fields-equals-PLANTED"],
        &["books", "list", "--limit", "s3cr3t-limit-PLANTED"],
        &["books", "show", "s3cr3t-ids-PLANTED"],
        &["books", "show", "1", "s3cr3t-second-id-PLANTED"],
        &["commands", "list", "s3cr3t-path-PLANTED"],
        &["books", "list", "--nope", "s3cr3t-unknown-flag-PLANTED"],
        &["books", "list", "--nope=s3cr3t-unknown-flag-equals-PLANTED"],
        &["--s3cr3t-flag-name-PLANTED"],
        &["help", "s3cr3t-topic-PLANTED"],
        &["s3cr3t-command-PLANTED"],
    ];

    for args in runs {
        let _ = shelf(&home, args);
    }

    let log = fs::read_to_string(home.join("state").join("shelf").join("usage.jsonl")).unwrap();
    assert_eq!(log.lines().count(), runs.len());
    for line in log.lines() {
        assert!(!line.contains("PLANTED"), "{line}");
    }
}
