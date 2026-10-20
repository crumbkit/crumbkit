//! Tests for temporary directories.

use std::fs;
use std::os::unix::fs::symlink;

use super::TempDir;

#[test]
fn a_temporary_directory_below_a_symbolic_link_has_a_path_without_the_link() {
    let base = TempDir::new("temp-root-link").unwrap();
    let real = base.create_dir("real").unwrap();
    let link = base.path().join("link");
    symlink(&real, &link).unwrap();

    let dir = TempDir::new_in(&link, "below-link").unwrap();

    assert_eq!(dir.path(), fs::canonicalize(dir.path()).unwrap());
    assert!(dir.path().starts_with(fs::canonicalize(&real).unwrap()));
}

#[test]
fn a_temporary_root_that_cannot_be_canonicalised_keeps_its_raw_path() {
    let base = TempDir::new("temp-root-missing").unwrap();
    let missing = base.path().join("missing");

    let dir = TempDir::new_in(&missing, "below-missing").unwrap();

    assert!(dir.path().starts_with(&missing));
    assert!(dir.path().is_dir());
}
