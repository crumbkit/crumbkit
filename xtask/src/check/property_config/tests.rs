//! Tests for the property test configuration check.

use super::check;
use crate::check::{File, Violation};

fn file(path: &str, contents: &str) -> File {
    File {
        path: path.to_owned(),
        contents: contents.to_owned(),
    }
}

#[test]
fn a_block_that_starts_with_the_shared_configuration_passes() {
    let contents = "use proptest::prelude::*;\n\nproptest! {\n    #![proptest_config(crate::property::config())]\n\n    #[test]\n    fn holds(x in 0..1_u8) {}\n}\n";

    assert_eq!(check(&[file("src/a/tests.rs", contents)]), []);
}

#[test]
fn a_block_without_the_shared_configuration_is_reported_with_its_line() {
    let contents =
        "use proptest::prelude::*;\n\nproptest! {\n    #[test]\n    fn holds(x in 0..1_u8) {}\n}\n";

    assert_eq!(
        check(&[file("src/a/tests.rs", contents)]),
        [Violation::UnconfiguredPropertyTests {
            path: "src/a/tests.rs".to_owned(),
            line: 3,
        }]
    );
}

#[test]
fn a_block_with_another_configuration_is_reported() {
    let contents = "    proptest::proptest! {\n        #![proptest_config(ProptestConfig::default())]\n    }\n";

    assert_eq!(
        check(&[file("src/a/tests.rs", contents)]),
        [Violation::UnconfiguredPropertyTests {
            path: "src/a/tests.rs".to_owned(),
            line: 1,
        }]
    );
}

#[test]
fn a_block_on_the_last_line_is_reported() {
    assert_eq!(
        check(&[file("src/a/tests.rs", "proptest! {")]),
        [Violation::UnconfiguredPropertyTests {
            path: "src/a/tests.rs".to_owned(),
            line: 1,
        }]
    );
}

#[test]
fn a_comment_that_names_the_macro_and_a_file_that_is_not_rust_are_ignored() {
    let comment = "// proptest! {\n/// proptest! {\n";

    assert_eq!(
        check(&[
            file("src/a.rs", comment),
            file("docs/notes.md", "proptest! {\n")
        ]),
        []
    );
}
