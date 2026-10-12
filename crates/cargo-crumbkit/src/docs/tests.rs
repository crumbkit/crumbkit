//! Tests for writing the reference.

use std::fs;
use std::path::Path;

use crumbkit::{Failure as _, Spelling};

use super::{DOCS, Written, write};

#[test]
fn the_written_file_prints_as_one_sentence_with_its_path_and_command_count() {
    let written = Written::new(Path::new("examples/shelf/docs/commands.md"), 14);

    assert_eq!(
        written.to_string(),
        "Wrote the reference of 14 commands to examples/shelf/docs/commands.md."
    );
}

#[test]
fn writing_creates_the_file_s_directory_and_replaces_an_older_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("docs").join("commands.md");

    write(&file, "# Old\n").unwrap();
    write(&file, "# New\n").unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "# New\n");
}

#[test]
fn a_file_that_cannot_be_written_fails_with_write_failed() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("README"), "A file, not a directory.\n").unwrap();
    let file = dir.path().join("README").join("commands.md");

    let error = write(&file, "# Commands\n").unwrap_err();

    assert_eq!(
        Spelling::COLONS.spell(error.code()),
        "crumbkit::docs::write_failed"
    );
    assert_eq!(
        error.to_string(),
        format!("the reference cannot be written to {}", file.display())
    );
}

#[test]
fn the_checked_in_schema_matches_the_docs_document_schema() {
    let generated =
        serde_json::to_string_pretty(&crumbkit::schemas::document_schema(DOCS)).unwrap() + "\n";
    let checked_in = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schemas/docs-v1.json"));

    assert_eq!(
        checked_in, generated,
        "schemas/docs-v1.json differs from the generated schema; replace the file with the generated schema"
    );
}
