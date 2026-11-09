//! The convention tests of reading, which hold its command tree, tool record,
//! help pages and examples to crumbkit's conventions with crumbkit-test.
//!
//! Each check reads the command tree and the tool record that reading builds
//! from the lists in `tests/fixtures/lists/`, so the checks cover the commands
//! that reading adds at run time. They live in an integration test so that the
//! example check can run the built binary, whose path cargo sets only for
//! integration tests. The example check runs each example in a new copy of the
//! fixture directory, with `READING_DIR` naming the copy, and compares each
//! run's standard output with its golden file under `tests/golden/examples/`
//! (ADR-0005). `SNAPSHOTS=overwrite cargo test` writes, rewrites and deletes
//! the golden files.

use std::path::Path;

use clap::Command;
use crumbkit::Tool;
use crumbkit_test::{
    Allow, Check, Examples, Failing, PathBudgets, Recoveries, Rule, Suite, Twin, Twins, Unprovoked,
};

/// The directory of reading's fixture lists.
const LISTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/lists");

/// The reason that the examples of each list's `edit` are not run.
const EDITOR: &str = "edit hands the terminal to the caller's editor, which waits for a person and prints no output to compare";

/// The reason that each list's `edit` has no `--yes`.
const SAVE: &str = "the person in the editor chooses whether to save the list, so there is nothing for --yes to confirm";

/// The reason that each list's `edit` has no `--expect`.
const WHOLE_FILE: &str = "the editor saves the whole file, and the person in the editor sees the list as it is when the editor opens it";

/// reading's opt-outs from crumbkit's conventions, which every check reads.
/// They name the `edit` command of each fixture list.
const ALLOW: &[Allow] = &[
    Allow::new(Rule::Examples, "finished edit", EDITOR),
    Allow::new(Rule::Examples, "to-read edit", EDITOR),
    Allow::new(Rule::Yes, "finished edit", SAVE),
    Allow::new(Rule::Yes, "to-read edit", SAVE),
    Allow::new(Rule::Expect, "finished edit", WHOLE_FILE),
    Allow::new(Rule::Expect, "to-read edit", WHOLE_FILE),
];

/// The budgets of the paths through reading's help.
const PATH_BUDGETS: PathBudgets<'_> = PathBudgets::new("tests/golden/examples").path(
    4_300,
    "reading's longest path, commands list, reads about 4,050 bytes, so 4,300 bytes leaves room for small changes and catches a page or an output that grows by much.",
);

/// reading's built binary, the directory that holds its example golden
/// files, and the fixture lists that each run reads. Each run starts in a
/// new copy of the fixture lists, and `READING_DIR` names that copy, so an
/// example of `entries set` that writes to a list leaves the fixture as it
/// is and the next run sees the list unchanged. The copy is why the run
/// provides `writable-lists`.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_reading"), "tests/golden/examples")
        .fresh_dir(LISTS)
        .env(&[("READING_DIR", ".")])
        .provides(&["writable-lists"])
        .allow(ALLOW);

/// Returns the command tree and the tool record that reading builds from the
/// lists in its fixture directory.
fn fixture_tool() -> (Command, Tool) {
    let lists = reading::lists_in(Path::new(LISTS));
    (reading::command(&lists), reading::tool(&lists))
}

/// `entries list` of a list that does not exist.
const UNREAD: Failing<'_> = Failing::new(
    &reading::NOT_FOUND,
    &["entries", "list", "--list", "unread"],
);

/// A dry run of `entries set` at a position past the end of the list.
const POSITION_7: Failing<'_> = Failing::new(
    &reading::OUT_OF_RANGE,
    &[
        "entries",
        "set",
        "--list",
        "to-read",
        "--position",
        "7",
        "--dry-run",
        "Emma",
    ],
);

/// reading's declared code that no command line in a test provokes, which
/// neither the recovery check nor the twin check needs a command line for.
const UNPROVOKED: &[Unprovoked<'_>] = &[Unprovoked::new(
    &reading::UNREADABLE,
    "Only a list file that exists and that reading cannot read provokes it, and a file's permissions differ between platforms.",
)];

/// The settings of reading's recovery check. A missing list, a position
/// past the end of a list and a version conflict each start a chain. Each
/// chain runs in its own copy of the fixture lists, and the last two run on
/// a dry run, so they write nothing to that copy either.
const RECOVERIES: Recoveries<'_> = Recoveries::new(EXAMPLES, &[reading::CODES])
    .starts(&[
        UNREAD,
        POSITION_7,
        Failing::new(
            &crumbkit::codes::VERSION_CONFLICT,
            &[
                "entries",
                "set",
                "--list",
                "to-read",
                "--position",
                "2",
                "--expect",
                "Emma",
                "--dry-run",
                "Mary Barton",
            ],
        ),
    ])
    .unprovoked(UNPROVOKED);

/// The settings of reading's twin check: one twin for each declared code
/// that a command line can provoke. Each twin differs from its refused line
/// in one word, so that word is the reason for the refusal (TEST-1).
const TWINS: Twins<'_> = Twins::new(EXAMPLES, &[reading::CODES])
    .pairs(&[
        Twin::new(UNREAD, &["entries", "list", "--list", "to-read"]),
        Twin::new(
            POSITION_7,
            &[
                "entries",
                "set",
                "--list",
                "to-read",
                "--position",
                "1",
                "--dry-run",
                "Emma",
            ],
        ),
    ])
    .unprovoked(UNPROVOKED);

/// Everything the checks read, which `conventions!` expands to one test for
/// each check. With examples set, the help checks read what the binary prints,
/// which reading needs because it builds part of its help from the lists it
/// finds at start.
const SUITE: Suite<'_> = Suite::new(fixture_tool)
    .allow(ALLOW)
    .verbs(&["stream", "edit", "set", "export"])
    .path_budgets(PATH_BUDGETS)
    .examples(EXAMPLES)
    .recoveries(RECOVERIES)
    .twins(TWINS)
    .skip(
        Check::CommandReference,
        "reading's commands come from the lists in the caller's directory, so no checked-in reference holds for every caller.",
    );

crumbkit_test::conventions!(SUITE);
