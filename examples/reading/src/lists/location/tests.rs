//! Tests for `reading where`.

use std::path::Path;

use super::Location;

#[test]
fn a_directory_that_reading_finds_prints_alone() {
    let location = Location::of(Some(Path::new("/home/ada/lists")));

    assert_eq!(location.to_string(), "/home/ada/lists");
}

#[test]
fn a_directory_that_reading_cannot_find_prints_why() {
    assert_eq!(
        Location::of(None).to_string(),
        "reading cannot find its lists, because READING_DIR is unset and the home directory is unknown."
    );
}
