//! End-to-end tests of `shelf self usage`. Each test runs the built binary
//! with XDG directories in a temporary directory of its own, so the runs
//! before `self usage` write the log that it reads (ADR-0029).
//!
//! The golden files hold `[..]` where the output names the day the report
//! starts or a wall time, which change from run to run, so these tests
//! compare without snapbox's `raw`, which would turn the wildcard off.

use std::fs;
use std::path::Path;

use serde_json::Value;
use snapbox::cmd::{Command, OutputAssert};
use snapbox::file;

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

/// Returns the path of shelf's usage log under `home`.
fn log(home: &Path) -> std::path::PathBuf {
    home.join("state").join("shelf").join("usage.jsonl")
}

/// Runs shelf under `home` in the ways that each finding of `self usage`
/// needs: three help pages before the first success, one id at a time, a
/// refused flag, a failure retried with the same words, and a cut-short
/// list followed by `--all`.
fn wasteful_runs(home: &Path) {
    let runs: [&[&str]; 11] = [
        &["-h"],
        &["books", "-h"],
        &["books", "list", "-h"],
        &["books", "show", "1"],
        &["books", "show", "2"],
        &["books", "show", "3"],
        &["books", "list", "--al"],
        &["books", "show", "7"],
        &["books", "show", "7"],
        &["books", "list"],
        &["books", "list", "--all"],
    ];
    for args in runs {
        let _ = shelf(home, args);
    }
}

#[test]
fn self_usage_prints_the_findings_of_the_earlier_runs_as_text() {
    let dir = tempfile::tempdir().unwrap();
    wasteful_runs(dir.path());

    shelf(dir.path(), &["self", "usage"])
        .success()
        .stdout_eq(file!["golden/self-usage.txt"])
        .stderr_eq("");
}

#[test]
fn self_usage_prints_the_findings_with_their_figures_and_the_summary_as_json() {
    let dir = tempfile::tempdir().unwrap();
    wasteful_runs(dir.path());

    shelf(dir.path(), &["self", "usage", "--json", "--full"])
        .success()
        .stdout_eq(file!["golden/self-usage-json-full.json"]);
}

/// Returns the hashes and trace ids in the usage log under `home`.
fn hashes(home: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut hashes = Vec::new();
    for line in fs::read_to_string(log(home))?.lines() {
        let line: Value = serde_json::from_str(line)?;
        hashes.extend(
            ["session", "args", "trace", "run"]
                .map(|field| line[field].as_str().unwrap_or_default().to_owned()),
        );
    }
    Ok(hashes)
}

/// Returns the standard output of `self usage` in text and in JSON with
/// every field under `home`.
fn reports(home: &Path) -> [String; 2] {
    [
        &["self", "usage"][..],
        &["self", "usage", "--json", "--full"],
    ]
    .map(|args| {
        let output = shelf(home, args).success().get_output().stdout.clone();
        String::from_utf8_lossy(&output).into_owned()
    })
}

#[test]
fn the_report_holds_no_value_that_a_caller_typed_and_no_hash() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    let runs: [&[&str]; 7] = [
        &["books", "list", "--fields", "s3cr3t-fields-PLANTED"],
        &["books", "list", "--limit", "s3cr3t-limit-PLANTED"],
        &["books", "show", "s3cr3t-ids-PLANTED"],
        &["commands", "list", "s3cr3t-path-PLANTED"],
        &["books", "list", "--s3cr3t-flag-PLANTED"],
        &["help", "s3cr3t-topic-PLANTED"],
        &["s3cr3t-command-PLANTED"],
    ];
    for args in runs {
        let _ = shelf(home, args);
    }

    let hashes = hashes(home).unwrap();
    for report in reports(home) {
        assert!(!report.contains("PLANTED"), "{report}");
        for hash in &hashes {
            assert!(!report.contains(hash.as_str()), "{hash} in {report}");
        }
    }
}

#[test]
fn records_edited_to_hold_other_text_are_left_out_and_counted() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    shelf(home, &["books", "list"]).success();
    let line = fs::read_to_string(log(home)).unwrap();
    let edits = [
        (
            r#""command":"books list""#,
            r#""command":"books s3cr3t-PLANTED""#,
        ),
        (r#""flags":[]"#, r#""flags":["--s3cr3t-PLANTED"]"#),
        (
            r#""exit":0"#,
            r#""exit":1,"code":"s3cr3t::PLANTED::value=hunter2""#,
        ),
        (r#""caller":"unknown""#, r#""caller":"s3cr3t-PLANTED""#),
    ];
    let mut edited = String::new();
    for (field, planted) in edits {
        let with_field = line.replace(r#""exit":0"#, r#""flags":[],"exit":0"#);
        let changed = with_field.replace(field, planted);
        assert_ne!(changed, with_field, "{field}");
        edited.push_str(&changed);
    }
    #[expect(
        clippy::disallowed_methods,
        reason = "this test owns its file writes, which edit the usage log"
    )]
    fs::write(log(home), [line, edited].concat()).unwrap();

    let [text, json] = reports(home);

    assert!(text.starts_with("1 run in 1 session since "), "{text}");
    assert!(
        text.contains(" 4 records were skipped because they name a command, flag, code or caller that the tool does not write.\n"),
        "{text}"
    );
    assert!(!text.contains("PLANTED"), "{text}");
    assert!(!json.contains("PLANTED"), "{json}");
    assert!(json.contains(r#""skipped":4"#), "{json}");
}
