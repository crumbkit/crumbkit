//! End-to-end tests of shelf's shell completion. Each test runs the built
//! binary as fish runs it for a Tab press, with `COMPLETE=fish` and the
//! command line after `--`, and XDG directories in a temporary directory of
//! its own (ADR-0048).

use std::path::Path;

use snapbox::cmd::Command;

/// Runs the built `shelf` binary as fish completes `words`, with a cleared
/// environment and XDG directories under `home`, and returns the first
/// column of each candidate line.
fn complete(home: &Path, words: &[&str]) -> Vec<String> {
    let output = Command::new(env!("CARGO_BIN_EXE_shelf"))
        .arg("--")
        .arg("shelf")
        .args(words)
        .env_clear()
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .env("COMPLETE", "fish")
        .assert()
        .success()
        .stderr_eq("")
        .get_output()
        .stdout
        .clone();
    String::from_utf8_lossy(&output)
        .lines()
        .map(|line| line.split('\t').next().unwrap_or(line).to_owned())
        .collect()
}

#[test]
fn completing_after_the_tool_name_offers_each_visible_resource_and_not_aliases() {
    let dir = tempfile::tempdir().unwrap();

    let offered = complete(dir.path(), &[""]);

    for resource in ["books", "commands", "schemas", "self"] {
        assert!(offered.iter().any(|word| word == resource), "{offered:?}");
    }
    assert!(!offered.iter().any(|word| word == "aliases"), "{offered:?}");
}

#[test]
fn completing_a_prefix_of_list_offers_list_and_not_its_hidden_alias_ls() {
    let dir = tempfile::tempdir().unwrap();

    let offered = complete(dir.path(), &["books", "l"]);

    assert_eq!(offered, ["list"]);
}

#[test]
fn a_prefix_that_matches_only_a_hidden_alias_completes_to_it() {
    let dir = tempfile::tempdir().unwrap();

    let offered = complete(dir.path(), &["books", "ls"]);

    assert_eq!(offered, ["ls"]);
}

#[test]
fn a_completion_run_writes_no_usage_line() {
    let dir = tempfile::tempdir().unwrap();

    complete(dir.path(), &["books", ""]);

    assert!(!dir.path().join("state").join("shelf").exists());
}
