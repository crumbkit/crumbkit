use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use super::{create, key};
use crate::usage::file::create_dir;
use crate::usage::testing::TempDir;

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
}

#[test]
fn a_run_that_loses_the_race_to_create_the_key_reads_the_winner_s_key() {
    let dir = TempDir::new("key-race");
    write_key(&dir, b"0123456789abcdef");

    assert_eq!(
        create(&dir.path().join("usage.key")),
        Some(*b"0123456789abcdef")
    );
}

#[test]
fn a_key_file_of_the_wrong_length_gives_no_key() {
    let dir = TempDir::new("key-length");
    write_key(&dir, b"too short");

    assert_eq!(key(dir.path()), None);
}
