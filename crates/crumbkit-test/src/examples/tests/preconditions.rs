//! Tests for the preconditions of examples, which a run of the check
//! provides or lacks.

use crumbkit::help::{Example, Guide, Leaf};
use snapbox::assert::Action;

use super::{examples, run};
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The guide of three examples: one without preconditions, one that needs
/// `docker`, and one that needs `docker` and `reports`.
const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[
    Example::new(&["echo books"], "Print a word"),
    Example::new(&["echo containers"], "Print the containers").needs(&["docker"]),
    Example::new(&["echo changes"], "Compare two reports").needs(&["docker", "reports"]),
])]);

/// The golden files of `GUIDE`, sorted by name.
const BOOKS: &str = "sh-c-echo-books.txt";
const CHANGES: &str = "sh-c-echo-changes.txt";
const CONTAINERS: &str = "sh-c-echo-containers.txt";

#[test]
fn a_run_that_provides_no_preconditions_skips_each_example_that_needs_one_and_names_what_it_lacks()
{
    let dir = TempDir::new("examples-needs-none").unwrap();

    let skipped = run(&GUIDE, &examples(&dir), Action::Overwrite);

    assert_eq!(dir.names(), [BOOKS]);
    assert_eq!(
        skipped,
        [
            "sh -c 'echo containers': the check skipped the example, because the run does not provide `docker`",
            "sh -c 'echo changes': the check skipped the example, because the run does not provide `docker`, `reports`",
        ]
    );
}

#[test]
fn a_run_runs_an_example_only_when_it_provides_every_precondition_of_the_example() {
    let dir = TempDir::new("examples-needs-some").unwrap();

    let skipped = run(
        &GUIDE,
        &examples(&dir).provides(&["docker"]),
        Action::Overwrite,
    );

    assert_eq!(dir.names(), [BOOKS, CONTAINERS]);
    assert_eq!(
        skipped,
        [
            "sh -c 'echo changes': the check skipped the example, because the run does not provide `reports`"
        ]
    );
}

#[test]
fn a_run_that_provides_every_precondition_runs_every_example() {
    let dir = TempDir::new("examples-needs-all").unwrap();

    let skipped = run(
        &GUIDE,
        &examples(&dir).provides(&["reports", "docker"]),
        Action::Overwrite,
    );

    assert_eq!(dir.names(), [BOOKS, CHANGES, CONTAINERS]);
    assert_eq!(skipped, Vec::<String>::new());
}

#[test]
fn a_skipped_example_keeps_its_golden_files_and_they_are_not_stale() {
    let dir = TempDir::new("examples-needs-keep").unwrap();
    let all = examples(&dir).provides(&["docker", "reports"]);
    run(&GUIDE, &all, Action::Overwrite);
    dir.write("sh-c-echo-containers.stderr", "a note\n");

    run(&GUIDE, &examples(&dir), Action::Overwrite);
    run(&GUIDE, &examples(&dir), Action::Verify);

    assert_eq!(
        dir.names(),
        [BOOKS, CHANGES, "sh-c-echo-containers.stderr", CONTAINERS]
    );
}

#[test]
fn a_skipped_example_still_counts_towards_golden_file_names_that_two_examples_share() {
    const SHARED: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[
        Example::new(&["echo books"], "Print a word"),
        Example::new(&["echo  books"], "Print the word again").needs(&["docker"]),
    ])]);
    let dir = TempDir::new("examples-needs-shared").unwrap();

    assert_eq!(
        panic_message(|| run(&SHARED, &examples(&dir), Action::Overwrite)).unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         sh -c 'echo  books': the example's golden file name `sh-c-echo-books.txt` is also the name for the example `sh -c 'echo books'`"
    );
}
