//! End-to-end tests that shelf's output names shelf, and never the library
//! that it is built on: the failure document, crumbkit's codes, the output
//! budget's variable and every golden file of the example tools.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use snapbox::cmd::{Command, OutputAssert};

/// Runs the built `shelf` binary with `args` and the variables `env`, in
/// `dir` with a cleared environment and XDG directories under it, and
/// returns its output for assertions.
fn shelf(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> OutputAssert {
    let mut command = Command::new(env!("CARGO_BIN_EXE_shelf"))
        .args(args)
        .current_dir(dir)
        .env_clear()
        .env("XDG_CONFIG_HOME", dir.join("config"))
        .env("XDG_DATA_HOME", dir.join("data"))
        .env("XDG_STATE_HOME", dir.join("state"))
        .env("XDG_CACHE_HOME", dir.join("cache"));
    for (name, value) in env {
        command = command.env(name, value);
    }
    command.assert()
}

/// Returns the JSON document that a run printed on standard output.
fn document(output: &OutputAssert) -> serde_json::Result<Value> {
    serde_json::from_slice(&output.get_output().stdout)
}

#[test]
fn a_refused_command_line_prints_a_failure_document_with_shelfs_schema_id_and_code() {
    let dir = tempfile::tempdir().unwrap();
    let output = shelf(dir.path(), &["books", "show", "--json"], &[]).code(2);
    let failure = document(&output).unwrap();
    assert_eq!(failure["schema"], "shelf/failure/v1");
    assert_eq!(failure["code"], "shelf::cli::usage");
}

#[test]
fn a_refused_command_line_prints_shelfs_code_in_text() {
    let dir = tempfile::tempdir().unwrap();
    let output = shelf(dir.path(), &["books", "show"], &[]).code(2);
    let err = String::from_utf8_lossy(&output.get_output().stderr).into_owned();
    assert!(err.contains("code:  shelf::cli::usage"), "{err}");
}

#[test]
fn the_output_budget_variable_takes_shelfs_name() {
    let dir = tempfile::tempdir().unwrap();
    let output = shelf(
        dir.path(),
        &["books", "show", "1", "2", "3", "4", "5", "6", "--json"],
        &[("SHELF_OUTPUT_BUDGET", "400")],
    );
    let batch = document(&output.code(1)).unwrap();
    assert_eq!(batch["failures"][0]["code"], "shelf::output::over_budget");
}

#[test]
fn the_old_output_budget_variable_has_no_effect() {
    let dir = tempfile::tempdir().unwrap();
    let output = shelf(
        dir.path(),
        &["books", "show", "1", "2", "3", "4", "5", "6", "--json"],
        &[("CRUMBKIT_OUTPUT_BUDGET", "400")],
    );
    let batch = document(&output.code(0)).unwrap();
    assert_eq!(batch["failures"], serde_json::json!([]));
}

/// Pushes every file below `dir` to `files`.
fn files_below(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            files_below(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

#[test]
fn no_golden_file_or_generated_reference_of_an_example_tool_names_crumbkit() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the examples directory");
    let mut files = Vec::new();
    for example in ["shelf", "reading", "tutorial"] {
        let golden = examples.join(example).join("tests").join("golden");
        if golden.is_dir() {
            files_below(&golden, &mut files).unwrap();
        }
    }
    files.push(examples.join("shelf").join("docs").join("commands.md"));
    assert!(files.len() > 100, "found only {} files", files.len());
    let naming: Vec<_> = files
        .iter()
        .filter(|path| {
            fs::read_to_string(path)
                .expect("a text file")
                .to_lowercase()
                .contains("crumbkit")
        })
        .map(|path| {
            path.strip_prefix(examples)
                .unwrap_or(path)
                .display()
                .to_string()
        })
        .collect();
    assert!(
        naming.is_empty(),
        "{} files name crumbkit:\n{}",
        naming.len(),
        naming.join("\n")
    );
}
