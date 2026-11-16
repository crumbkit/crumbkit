//! End-to-end tests of the usage log when the reader of standard output
//! closes the pipe part way through a result. The test runs the built
//! binary with XDG directories in a temporary directory of its own, on a
//! usage log that is much larger than a pipe holds, and reads the line that
//! the run appended to `shelf/usage.jsonl` there.

#![expect(
    clippy::unwrap_used,
    clippy::disallowed_methods,
    reason = "The tests spawn the built binary and seed its log, and a failing helper should stop the test"
)]

use std::fs;
use std::io::Read as _;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

/// The most that a pipe holds when its reader is not reading, with room for
/// the buffer of the standard library's stdout in front of it.
const PIPE_ROOM: u64 = 128 * 1024;

/// The number of copies of one usage line that the seeded log holds, which
/// makes each command's result several times the size of a pipe.
const LINES: usize = 3000;

/// Returns a shelf command with `args`, a cleared environment and XDG
/// directories under `home`.
fn shelf(home: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_shelf"));
    command
        .args(args)
        .env_clear()
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"));
    command
}

/// Returns the path of shelf's usage log under `home`.
fn log(home: &Path) -> std::path::PathBuf {
    home.join("state").join("shelf").join("usage.jsonl")
}

/// Runs one command so that the log holds a line, then replaces the log with
/// `LINES` copies of that line, each with a run id of its own.
fn seed(home: &Path) {
    let run = shelf(home, &["books", "list", "--json"]).output().unwrap();
    assert!(run.status.success());
    let text = fs::read_to_string(log(home)).unwrap();
    let mut line: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    let mut seeded = String::new();
    for number in 0..LINES {
        line["run"] = Value::String(format!("{number:016x}"));
        seeded.push_str(&line.to_string());
        seeded.push('\n');
    }
    fs::write(log(home), seeded).unwrap();
}

/// Returns the number of bytes that a complete run of `args` prints.
fn full_size(home: &Path, args: &[&str]) -> u64 {
    let run = shelf(home, args).output().unwrap();
    run.stdout.len() as u64
}

/// Runs `args` with a reader that takes `taken` bytes and closes the pipe,
/// and returns the exit code of the run and the `out_bytes` of the line that
/// the run appended to the log.
fn closed_after(home: &Path, args: &[&str], taken: usize) -> (Option<i32>, u64) {
    let mut child = shelf(home, args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let mut head = vec![0; taken];
    stdout.read_exact(&mut head).unwrap();
    drop(stdout);
    let status = child.wait().unwrap();
    let text = fs::read_to_string(log(home)).unwrap();
    let last: Value = serde_json::from_str(text.lines().last().unwrap()).unwrap();
    assert_eq!(
        last["command"],
        args.iter()
            .filter(|word| !word.starts_with("--"))
            .copied()
            .collect::<Vec<_>>()
            .join(" ")
    );
    (status.code(), last["out_bytes"].as_u64().unwrap())
}

#[test]
fn a_result_whose_reader_closed_the_pipe_records_a_count_of_at_most_the_results_size() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path();
    seed(home);
    let args = ["self", "runs", "list", "--all", "--json"];
    let full = full_size(home, &args);
    assert!(full > 2 * PIPE_ROOM, "{full}");

    let (_, out_bytes) = closed_after(home, &args, 100);

    assert!(out_bytes <= 100 + PIPE_ROOM, "{out_bytes}");
    assert!(out_bytes < full, "{out_bytes} of {full}");
}
