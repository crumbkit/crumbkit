//! Tests for the output weight.

use std::fs;
use std::path::Path;

use crumbkit::{Failure as _, Spelling};

use super::largest;

/// Writes `contents` to the file `name` in `dir`.
fn write(dir: &Path, name: &str, contents: &str) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests own their golden files, which they write into a temporary directory"
    )]
    fs::write(dir.join(name), contents).unwrap();
}

/// The command lines of the two examples of `shelf books list`.
fn examples() -> Vec<String> {
    vec![
        "shelf books list".to_owned(),
        "shelf books list --all".to_owned(),
    ]
}

#[test]
fn the_output_weight_is_the_size_of_the_largest_golden_file_of_the_examples() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "shelf-books-list.txt", "four\n");
    write(dir.path(), "shelf-books-list-all.txt", "every book\n");
    write(
        dir.path(),
        "shelf-books-show-2-5.txt",
        "the largest file of all\n",
    );

    assert_eq!(largest(dir.path(), &examples()).unwrap(), Some(11));
}

#[test]
fn examples_without_golden_files_have_no_output_weight() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "shelf-books-list-all.txt", "every book\n");

    assert_eq!(largest(dir.path(), &examples()).unwrap(), Some(11));
    assert_eq!(
        largest(&dir.path().join("missing"), &examples()).unwrap(),
        None
    );
    assert_eq!(largest(dir.path(), &[]).unwrap(), None);
}

#[test]
fn a_golden_directory_that_is_a_file_fails_with_golden_unreadable() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "goldens", "not a directory\n");

    let error = largest(&dir.path().join("goldens"), &examples()).unwrap_err();

    assert_eq!(
        Spelling::COLONS.spell(error.code()),
        "crumbkit::weigh::golden_unreadable"
    );
}
