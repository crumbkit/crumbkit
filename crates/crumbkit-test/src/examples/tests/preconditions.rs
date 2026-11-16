//! Tests for the preconditions of examples, which a run of the check
//! provides or lacks.

use crumbkit::Tool;
use crumbkit::help::{Example, Help, Leaf};
use snapbox::assert::Action;

use super::{examples, run};
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The tool record of three examples: one without preconditions, one that needs
/// `docker`, and one that needs `docker` and `reports`.
const TOOL: Tool = Tool::new().help(Help::new().leaves(&[Leaf::new(&["-c"]).examples(&[
    Example::new(&["echo books"], "Print a word"),
    Example::new(&["echo containers"], "Print the containers").needs(&["docker"]),
    Example::new(&["echo changes"], "Compare two reports").needs(&["docker", "reports"]),
])]));

/// The golden files of `TOOL`, sorted by name.
const BOOKS: &str = "sh-c-echo-books.txt";
const CHANGES: &str = "sh-c-echo-changes.txt";
const CONTAINERS: &str = "sh-c-echo-containers.txt";

#[test]
fn a_run_that_provides_no_preconditions_skips_each_example_that_needs_one_and_names_what_it_lacks()
{
    let dir = TempDir::new("examples-needs-none").unwrap();

    let skipped = run(&TOOL, &examples(&dir), Action::Overwrite);

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
        &TOOL,
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
        &TOOL,
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
    run(&TOOL, &all, Action::Overwrite);
    dir.write("sh-c-echo-containers.stderr", "a note\n");

    run(&TOOL, &examples(&dir), Action::Overwrite);
    run(&TOOL, &examples(&dir), Action::Verify);

    assert_eq!(
        dir.names(),
        [BOOKS, CHANGES, "sh-c-echo-containers.stderr", CONTAINERS]
    );
}

#[test]
fn a_skipped_example_still_counts_towards_golden_file_names_that_two_examples_share() {
    const SHARED: Tool = Tool::new().help(Help::new().leaves(&[Leaf::new(&["-c"]).examples(&[
        Example::new(&["echo books"], "Print a word"),
        Example::new(&["echo  books"], "Print the word again").needs(&["docker"]),
    ])]));
    let dir = TempDir::new("examples-needs-shared").unwrap();

    assert_eq!(
        panic_message(|| run(&SHARED, &examples(&dir), Action::Overwrite)).unwrap(),
        "the command tree breaks crumbkit's example rules:\n\
         sh -c 'echo  books': the example's golden file name `sh-c-echo-books.txt` is also the name for the example `sh -c 'echo books'`"
    );
}

/// The tool record of one example that writes to standard error and needs
/// `docker`, and one that needs nothing, so a run without `docker` has work
/// to do between its skip and its stale scan.
const NOISY: Tool =
    Tool::new().help(
        Help::new().leaves(&[Leaf::new(&["-c"]).examples(&[
            Example::new(&["echo containers; echo note >&2"], "Print the containers")
                .needs(&["docker"]),
            Example::new(&["echo books"], "Print a word"),
            Example::new(&["echo authors"], "Print another word"),
            Example::new(&["echo shelves"], "Print a third word"),
        ])]),
    );

#[test]
fn a_run_that_skips_an_example_never_deletes_the_golden_file_that_a_run_in_parallel_writes_for_it()
{
    for round in 0..20 {
        let dir = TempDir::new(&format!("examples-needs-parallel-{round}")).unwrap();

        std::thread::scope(|scope| {
            let with = scope.spawn(|| {
                run(
                    &NOISY,
                    &examples(&dir).provides(&["docker"]),
                    Action::Overwrite,
                )
            });
            let without = scope.spawn(|| run(&NOISY, &examples(&dir), Action::Overwrite));
            with.join().unwrap();
            without.join().unwrap();
        });

        // Both orders of the two runs end with the three goldens of the
        // examples that run everywhere and the two of the `docker` example.
        assert_eq!(
            dir.names(),
            [
                "sh-c-echo-authors.txt",
                "sh-c-echo-books.txt",
                "sh-c-echo-containers-echo-note-2.stderr",
                "sh-c-echo-containers-echo-note-2.txt",
                "sh-c-echo-shelves.txt",
            ],
            "round {round}"
        );
    }
}
