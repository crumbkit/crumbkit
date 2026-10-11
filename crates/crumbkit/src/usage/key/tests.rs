use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use super::{create, key};
use crate::usage::file::create_dir;
use crate::usage::testing::TempDir;

/// Returns the names of the files in `dir`, sorted.
fn files(dir: &TempDir) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// Writes `bytes` as the key file in `dir`.
fn write_key(dir: &TempDir, bytes: &[u8]) {
    create_dir(dir.path()).unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes the key file that the usage log then reads"
    )]
    fs::write(dir.path().join("usage.key"), bytes).unwrap();
}

#[test]
fn the_first_run_creates_a_16_byte_key_with_mode_0600_and_later_runs_reuse_it() {
    let dir = TempDir::new("key-created");
    create_dir(dir.path()).unwrap();

    let first = key(dir.path()).unwrap();
    let second = key(dir.path()).unwrap();

    assert_eq!(first, second);
    let file = dir.path().join("usage.key");
    assert_eq!(fs::read(&file).unwrap(), first);
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(files(&dir), ["usage.key"]);
}

#[test]
fn a_run_that_loses_the_race_to_create_the_key_reads_the_winner_s_key() {
    let dir = TempDir::new("key-race");
    write_key(&dir, b"0123456789abcdef");

    assert_eq!(create(dir.path()), Some(*b"0123456789abcdef"));
    assert_eq!(files(&dir), ["usage.key"]);
}

#[test]
fn an_empty_key_file_is_replaced_with_a_new_key_that_later_runs_reuse() {
    let dir = TempDir::new("key-empty");
    write_key(&dir, b"");

    let replaced = key(dir.path()).unwrap();

    assert_eq!(key(dir.path()), Some(replaced));
    assert_eq!(fs::read(dir.path().join("usage.key")).unwrap(), replaced);
    assert_eq!(files(&dir), ["usage.key"]);
}

#[test]
fn a_key_file_of_the_wrong_length_is_replaced_with_a_new_key() {
    let dir = TempDir::new("key-length");
    write_key(&dir, b"too short");

    let replaced = key(dir.path()).unwrap();

    assert_eq!(fs::read(dir.path().join("usage.key")).unwrap(), replaced);
}
