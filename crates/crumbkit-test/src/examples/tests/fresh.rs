//! Tests for the fixture of `Examples::fresh_dir`, which the example check
//! copies into a new working directory for each run.

use std::fs;

use crumbkit::help::{Example, Guide, Leaf};
use snapbox::assert::Action;

use super::{examples, run};
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The guide of two examples: the first lists the working directory and
/// writes a file there, and the second lists it again.
const WRITE: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[
    Example::new(&["ls; touch made"], "List and write"),
    Example::new(&["ls"], "List"),
])]);

/// Returns a fixture that holds the file `marker`.
fn fixture(label: &str) -> TempDir {
    let fixture = TempDir::new(label).unwrap();
    fixture.write("marker", "");
    fixture
}

#[test]
fn each_example_starts_in_its_own_copy_of_the_fresh_dir_fixture_and_sees_no_file_that_an_earlier_example_wrote()
 {
    let dir = TempDir::new("examples-fresh").unwrap();
    let fixture = fixture("examples-fresh-fixture");
    let path = fixture.path().display().to_string();

    run(&WRITE, &examples(&dir).fresh_dir(&path), Action::Overwrite);

    let first = fs::read_to_string(dir.path().join("sh-c-ls-touch-made.txt")).unwrap();
    let second = fs::read_to_string(dir.path().join("sh-c-ls.txt")).unwrap();
    assert_eq!(first, "marker\n");
    assert_eq!(second, "marker\n");
    assert_eq!(fixture.names(), ["marker"]);
}

#[test]
fn the_fresh_directory_and_its_canonical_path_are_redacted_as_root() {
    const PWD: Guide = Guide::new()
        .leaves(&[Leaf::new(&["-c"])
            .examples(&[Example::new(&["pwd -L; pwd -P"], "Print the directory")])]);
    let dir = TempDir::new("examples-fresh-root").unwrap();
    let fixture = fixture("examples-fresh-root-fixture");
    let path = fixture.path().display().to_string();

    run(&PWD, &examples(&dir).fresh_dir(&path), Action::Overwrite);

    let output = fs::read_to_string(dir.path().join("sh-c-pwd-L-pwd-P.txt")).unwrap();
    assert_eq!(output, "[ROOT]\n[ROOT]\n");
}

#[test]
fn a_fresh_dir_fixture_that_is_not_a_directory_fails_the_check_before_any_run() {
    let dir = TempDir::new("examples-fresh-missing").unwrap();

    let message = panic_message(|| {
        run(
            &WRITE,
            &examples(&dir).fresh_dir("/nonexistent/fixture"),
            Action::Overwrite,
        );
    });

    assert_eq!(
        message.unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         Examples::fresh_dir names `/nonexistent/fixture`, which is not a directory, so the check runs no example"
    );
    assert_eq!(dir.names(), Vec::<String>::new());
}

#[test]
fn a_symbolic_link_in_the_fresh_dir_fixture_fails_the_run() {
    let dir = TempDir::new("examples-fresh-link").unwrap();
    let fixture = fixture("examples-fresh-link-fixture");
    let link = fixture.path().join("link");
    std::os::unix::fs::symlink("marker", &link).unwrap();
    let path = fixture.path().display().to_string();

    let message =
        panic_message(|| run(&WRITE, &examples(&dir).fresh_dir(&path), Action::Overwrite)).unwrap();

    assert!(
        message.contains(&format!(
            "the fixture `{path}` of Examples::fresh_dir cannot be copied: `{}` is a symbolic link, which the check does not copy",
            link.display()
        )),
        "{message}"
    );
}

#[test]
fn fresh_dir_and_current_dir_replace_each_other() {
    const LS: Guide =
        Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(&["ls"], "List")])]);
    let dir = TempDir::new("examples-fresh-replace").unwrap();
    let fixture = fixture("examples-fresh-replace-fixture");
    let work = TempDir::new("examples-fresh-replace-work").unwrap();
    work.write("other", "");
    let fixture_path = fixture.path().display().to_string();
    let work_path = work.path().display().to_string();

    run(
        &LS,
        &examples(&dir)
            .current_dir(&work_path)
            .fresh_dir(&fixture_path),
        Action::Overwrite,
    );
    let fresh = fs::read_to_string(dir.path().join("sh-c-ls.txt")).unwrap();
    run(
        &LS,
        &examples(&dir)
            .fresh_dir(&fixture_path)
            .current_dir(&work_path),
        Action::Overwrite,
    );
    let fixed = fs::read_to_string(dir.path().join("sh-c-ls.txt")).unwrap();

    assert_eq!(fresh, "marker\n");
    assert_eq!(fixed, "other\n");
    assert_eq!(work.names(), ["other"]);
}
