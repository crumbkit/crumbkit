//! The example test of shelf. It runs every example in shelf's guide against
//! the built binary and compares each run's standard output with its golden
//! file under `tests/golden/examples/` (ADR-0005). `SNAPSHOTS=overwrite cargo
//! test` writes, rewrites and deletes those files.

use clap::CommandFactory as _;
use crumbkit_test::Examples;
use shelf::{Cli, GUIDE};

/// shelf's built binary and the directory that holds its example golden
/// files. The examples of `self usage` are exempt, because its output names
/// the day the report starts, which changes every day; the end-to-end tests
/// in `tests/usage.rs` check its output with the day left out.
const EXAMPLES: Examples<'_> =
    Examples::new(env!("CARGO_BIN_EXE_shelf"), "tests/golden/examples").exempt(&["self usage"]);

#[test]
fn every_example_in_the_guide_runs_and_prints_its_golden_output() {
    crumbkit_test::check_examples(&Cli::command(), &GUIDE, &EXAMPLES);
}
