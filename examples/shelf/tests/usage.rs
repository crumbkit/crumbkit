//! End-to-end tests of shelf's usage log. Each test runs the built binary
//! with XDG directories in a temporary directory of its own, which is
//! removed when the test ends, and reads the lines that the runs appended to
//! `shelf/usage.jsonl` there (ADR-0029).

use std::fs;
use std::path::Path;

use clap::CommandFactory as _;
use serde_json::{Value, json};
use shelf::Cli;
use snapbox::cmd::{Command, OutputAssert};

/// Runs the built `shelf` binary with `args`, a cleared environment and XDG
/// directories under `home`, and returns its output for assertions.
fn shelf(home: &Path, args: &[&str]) -> OutputAssert {
    shelf_with(home, args, &[])
}

/// Runs shelf as [`shelf`] does, with the variables `env` added to its
/// environment.
fn shelf_with(home: &Path, args: &[&str], env: &[(&str, &str)]) -> OutputAssert {
    let mut command = Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .env_clear()
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"));
    for (name, value) in env {
        command = command.env(name, value);
    }
    command.assert()
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
    let hex = |value: &Value, len: usize| {
        value.as_str().is_some_and(|text| {
            text.len() == len
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
    assert!(hex(&line["session"], 16), "{line}");
    assert!(hex(&line["args"], 16), "{line}");
    assert!(hex(&line["trace"], 32), "{line}");
    assert!(hex(&line["run"], 16), "{line}");
    for field in ["at", "ms", "session", "args", "trace", "run"] {
        line[field] = json!("<checked>");
    }
    line
}

#[test]
fn a_run_appends_one_line_with_its_command_flags_exit_code_and_output_size() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    let run = shelf(home, &["books", "list", "--limit", "2", "--json"]).success();

    let out_bytes = run.get_output().stdout.len();
    let lines = lines(home).unwrap();
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
            "trace": "<checked>",
            "run": "<checked>",
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
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    shelf(home, &["books", "list", "--al"]).code(2);

    let line = stable(lines(home).unwrap().remove(0));
    assert_eq!(line["command"], "books list");
    assert_eq!(line["unknown_flags"], 1);
    assert_eq!(line["nearest"], "--all");
    assert_eq!(line["exit"], 2);
    assert_eq!(line["code"], "crumbkit::cli::usage");
    assert_eq!(line.get("flags"), None);
}

#[test]
fn a_request_for_help_records_the_command_and_which_page() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    shelf(home, &["help", "books", "show"]).success();
    shelf(home, &["books", "-h"]).success();

    let lines = lines(home).unwrap();
    assert_eq!(lines[0]["help"], "long");
    assert_eq!(lines[0]["command"], "books show");
    assert_eq!(lines[1]["help"], "short");
    assert_eq!(lines[1]["command"], "books");
    assert_eq!(lines[1]["exit"], 0);
}

#[test]
fn a_run_of_an_alias_records_the_alias_and_the_command_it_expanded_to() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
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

    shelf(home, &["firsts", "--limit", "1"]).success();

    let line = stable(lines(home).unwrap().remove(0));
    assert_eq!(line["alias"], true);
    assert_eq!(line["command"], "books list");
    assert_eq!(line["flags"], json!(["--limit", "--long"]));
}

#[test]
fn a_run_with_a_valid_traceparent_records_the_caller_s_trace_and_names_the_caller_as_parent() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    shelf_with(home, &["books", "list"], &[("TRACEPARENT", traceparent)]).success();

    let line = lines(home).unwrap().remove(0);
    assert_eq!(line["trace"], "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(line["parent"], "00f067aa0ba902b7");
    assert_ne!(line["run"], "00f067aa0ba902b7");
}

#[test]
fn runs_without_a_traceparent_each_start_a_new_trace_with_no_parent() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();

    shelf(home, &["books", "list"]).success();
    shelf(home, &["books", "list"]).success();

    let lines = lines(home).unwrap();
    for line in &lines {
        assert_eq!(stable(line.clone()).get("parent"), None);
    }
    assert_ne!(lines[0]["trace"], lines[1]["trace"]);
    assert_ne!(lines[0]["run"], lines[1]["run"]);
}

#[test]
fn no_line_holds_a_value_that_the_caller_typed() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
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

    // The caller's trace context: a planted TRACEPARENT, which starts a new
    // trace, and a planted TRACESTATE beside both an invalid and a valid
    // TRACEPARENT.
    let contexts: [&[(&str, &str)]; 3] = [
        &[("TRACEPARENT", "00-s3cr3t-traceparent-PLANTED-01")],
        &[
            ("TRACEPARENT", "s3cr3t-PLANTED"),
            ("TRACESTATE", "vendor=s3cr3t-tracestate-PLANTED"),
        ],
        &[
            (
                "TRACEPARENT",
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            ),
            ("TRACESTATE", "vendor=s3cr3t-tracestate-PLANTED"),
        ],
    ];

    for args in runs {
        let _ = shelf(home, args);
    }
    for env in contexts {
        let _ = shelf_with(home, &["books", "list"], env);
    }

    let log = fs::read_to_string(home.join("state").join("shelf").join("usage.jsonl")).unwrap();
    assert_eq!(log.lines().count(), runs.len() + contexts.len());
    for line in log.lines() {
        assert!(!line.contains("PLANTED"), "{line}");
    }
}
