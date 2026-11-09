//! The convention tests of shelf, which hold its command tree, tool record,
//! help pages and examples to crumbkit's conventions with crumbkit-test.
//!
//! They live in an integration test so that the example check can run the
//! built binary, whose path cargo sets only for integration tests, and so
//! that every check reads the one list of shelf's opt-outs. The example
//! check compares each run's standard output with its golden file under
//! `tests/golden/examples/` (ADR-0005). `SNAPSHOTS=overwrite cargo test`
//! writes, rewrites and deletes the golden files.

use clap::CommandFactory as _;
use crumbkit_test::{
    Allow, Examples, Failing, PathBudgets, Recoveries, Rule, Suite, Twin, Twins, Unprovoked,
};
use shelf::{Cli, TOOL};

/// The reason that the example check skips the examples of `self usage` and
/// `self figures list`.
const DAILY: &str = "The output names the day the report starts, which changes every day, and tests/self_usage.rs checks the output without the day.";

/// shelf's opt-outs from crumbkit's conventions, which every check reads.
const ALLOW: &[Allow] = &[
    Allow::new(Rule::Examples, "self usage", DAILY),
    Allow::new(Rule::Examples, "self figures list", DAILY),
    Allow::new(
        Rule::PersonalAliases,
        "",
        "A person who runs shelf by hand sees the aliases they wrote on the root's help, and the help checks print it without them.",
    ),
];

/// The budgets of the paths through shelf's help. shelf's longest path,
/// `schemas show`, reads about 4,850 bytes.
const PATH_BUDGETS: PathBudgets<'_> = PathBudgets::new("tests/golden/examples").path(
    5_300,
    "shelf's longest path, schemas show, reads about 4,850 bytes, so 5,300 bytes leaves room for small changes and catches a page or an output that grows by much.",
);

/// shelf's built binary and the directory that holds its example golden
/// files.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_shelf"), "tests/golden/examples").allow(ALLOW);

/// `books show` of a book that does not exist.
const SHOW_7: Failing<'_> = Failing::new(&shelf::NOT_FOUND, &["books", "show", "7"]);

/// `books show --from` of an ids file that does not exist.
const MISSING_FILE: Failing<'_> = Failing::new(
    &shelf::UNREADABLE_IDS_FILE,
    &["books", "show", "--from", "tests/fixtures/ids/missing.txt"],
);

/// `books show --from` of an ids file with a line that is not an id.
const BAD_LINE: Failing<'_> = Failing::new(
    &shelf::BAD_IDS_FILE,
    &["books", "show", "--from", "tests/fixtures/ids/bad-line.txt"],
);

/// `books show --from` of an ids file with 101 ids.
const TOO_MANY: Failing<'_> = Failing::new(
    &shelf::TOO_MANY_IDS,
    &["books", "show", "--from", "tests/fixtures/ids/101-ids.txt"],
);

/// `books show --from` of an empty ids file.
const EMPTY_FILE: Failing<'_> = Failing::new(
    &shelf::EMPTY_IDS_FILE,
    &["books", "show", "--from", "tests/fixtures/ids/empty.txt"],
);

/// The words of `books show --from` of an ids file that holds the id 2,
/// which each `--from` line above differs from in the file alone.
const ONE_ID: &[&str] = &["books", "show", "--from", "tests/fixtures/ids/one-id.txt"];

/// The command lines that start shelf's recovery chains. Each `--from`
/// start reads a file under `tests/fixtures/ids/`, and its alternative
/// `shelf books show <ID>...` runs with the id 2. The last start lowers the
/// output budget, so `books show` leaves out ids and its failure runs
/// `shelf books show <ID>` with the first id left out.
const STARTS: &[Failing<'_>] = &[
    SHOW_7,
    MISSING_FILE,
    BAD_LINE,
    TOO_MANY,
    EMPTY_FILE,
    Failing::new(
        &crumbkit::codes::CONFIRM_NEEDS_YES,
        &["books", "remove", "2"],
    ),
    Failing::new(
        &crumbkit::codes::CLI_USAGE,
        &["books", "list", "--limit", "101"],
    ),
    Failing::new(&crumbkit::codes::CLI_USAGE, &["books", "show"]),
    Failing::new(
        &crumbkit::codes::OUTPUT_OVER_BUDGET,
        &["books", "show", "1", "2", "3", "4", "5", "6"],
    )
    .env(&[("SHELF_OUTPUT_BUDGET", Some("400"))]),
];

/// shelf's declared codes that no command line in a test provokes, which
/// neither the recovery check nor the twin check needs a command line for.
const UNPROVOKED: &[Unprovoked<'_>] = &[
    Unprovoked::new(
        &shelf::REMOVED_FILE,
        "Only a file of removed books that shelf cannot read or write provokes it, such as one on a full disk.",
    ),
    Unprovoked::new(
        &shelf::NO_STATE_DIRECTORY,
        "The check always sets XDG_STATE_HOME, and only a run without it and without HOME provokes the code.",
    ),
];

/// The settings of shelf's recovery check.
const RECOVERIES: Recoveries<'_> = Recoveries::new(EXAMPLES, &[shelf::CODES])
    .starts(STARTS)
    .unprovoked(UNPROVOKED)
    .values(&[("ID", "2")]);

/// The settings of shelf's twin check: one twin for each declared code that
/// a command line can provoke. Each twin differs from its refused line in
/// one word, so that word is the reason for the refusal (TEST-1).
const TWINS: Twins<'_> = Twins::new(EXAMPLES, &[shelf::CODES])
    .pairs(&[
        Twin::new(SHOW_7, &["books", "show", "2"]),
        Twin::new(MISSING_FILE, ONE_ID),
        Twin::new(BAD_LINE, ONE_ID),
        Twin::new(TOO_MANY, ONE_ID),
        Twin::new(EMPTY_FILE, ONE_ID),
    ])
    .unprovoked(UNPROVOKED);

/// Everything the checks read, which `conventions!` expands to one test for
/// each check.
const SUITE: Suite<'_> = Suite::new(Cli::command, &TOOL)
    .allow(ALLOW)
    .verbs(&["summarize", "usage", "remove"])
    .path_budgets(PATH_BUDGETS)
    .examples(EXAMPLES)
    .codes(&[shelf::CODES])
    .recoveries(RECOVERIES)
    .twins(TWINS);

crumbkit_test::conventions!(SUITE);
