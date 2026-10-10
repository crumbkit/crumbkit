use std::fs::{self, File};
use std::io;
use std::os::unix::fs::PermissionsExt as _;
use std::time::Duration;

use super::{Limits, append, create_dir};
use crate::usage::testing::TempDir;

/// Limits that a test reaches without writing 10 MB or waiting 100 ms.
const SMALL: Limits = Limits {
    size: 20,
    keep: 2,
    wait: Duration::from_millis(20),
};

/// Returns the text of the log in `dir`.
fn log(dir: &TempDir) -> String {
    fs::read_to_string(dir.path().join("usage.jsonl")).unwrap()
}

#[test]
fn the_directory_has_mode_0700_and_the_log_mode_0600() {
    let dir = TempDir::new("modes");

    create_dir(dir.path()).unwrap();
    append(dir.path(), b"{}\n", &SMALL).unwrap();

    let mode = |path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(dir.path().to_owned()), 0o700);
    assert_eq!(mode(dir.path().join("usage.jsonl")), 0o600);
}

#[test]
fn each_append_adds_its_line_at_the_end() {
    let dir = TempDir::new("append");
    create_dir(dir.path()).unwrap();

    append(dir.path(), b"one\n", &SMALL).unwrap();
    append(dir.path(), b"two\n", &SMALL).unwrap();

    assert_eq!(log(&dir), "one\ntwo\n");
}

#[test]
fn past_the_size_limit_the_log_keeps_its_newest_lines_and_appends_the_new_one() {
    let dir = TempDir::new("trim");
    create_dir(dir.path()).unwrap();
    for line in ["line-1\n", "line-2\n", "line-3\n", "line-4\n"] {
        append(dir.path(), line.as_bytes(), &SMALL).unwrap();
    }

    // The log held 21 bytes, past the limit of 20, before the fourth line, so
    // it kept the newest 2 lines and appended the fourth.
    assert_eq!(log(&dir), "line-2\nline-3\nline-4\n");
}

#[test]
fn a_lock_held_past_the_wait_drops_the_line() {
    let dir = TempDir::new("lock");
    create_dir(dir.path()).unwrap();
    append(dir.path(), b"before\n", &SMALL).unwrap();
    let holder = File::open(dir.path().join("usage.jsonl")).unwrap();
    holder.lock().unwrap();

    let error = append(dir.path(), b"dropped\n", &SMALL).unwrap_err();

    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert_eq!(log(&dir), "before\n");
}
