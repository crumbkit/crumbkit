//! Tests for the fixture of `Examples::fresh_dir`, which the recovery check
//! copies once for each chain.

use snapbox::assert::Action;

use super::{CODES, NOT_FOUND, OTHERS, Setup, check_with, failure, problems, run};
use crate::examples::Examples;
use crate::recoveries::{Failing, Recoveries};
use crate::temp::TempDir;

/// Returns a tool whose `books show 7` fails with `NOT_FOUND` and writes the
/// file `made` in its working directory, unless `made` is there already,
/// and whose `books list` succeeds only when the working directory holds
/// `marker` and `made`.
fn tool(label: &str) -> Setup {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let dir = TempDir::new(label).unwrap();
    dir.write_script(
        "tool",
        &format!(
            r#"#!/bin/sh
case "$*" in
  'books show 7 --json') if [ -f made ]; then exit 3; fi; touch made; printf '%s' '{show}'; exit 1;;
  'books list --json') if [ -f marker ] && [ -f made ]; then echo '{{}}'; exit 0; fi; exit 3;;
  *) exit 3;;
esac
"#
        ),
    );
    let script = dir.path().join("tool").display().to_string();
    Setup { dir, script }
}

/// Returns a directory that holds the file `marker`, with its path.
fn marked(label: &str) -> (TempDir, String) {
    let dir = TempDir::new(label).unwrap();
    dir.write("marker", "");
    let path = dir.path().display().to_string();
    (dir, path)
}

#[test]
fn the_runs_of_one_recovery_chain_share_one_copy_of_the_fresh_dir_fixture_and_the_next_chain_gets_its_own()
 {
    let setup = tool("recoveries-fresh");
    let (fixture, path) = marked("recoveries-fresh-fixture");
    let starts = [
        Failing::new(&NOT_FOUND, &["books", "show", "7"]),
        Failing::new(&NOT_FOUND, &["books", "show", "7"]),
    ];
    let examples = Examples::new(&setup.script, "").fresh_dir(&path);
    let recoveries = Recoveries::new(examples, CODES)
        .starts(&starts)
        .unprovoked(OTHERS);

    check_with(&setup, &recoveries, Action::Overwrite);

    let chain = "tool books show 7\n\
                 \x20 fails with tool::books::not_found\n\
                 \x20 run tool books list\n\
                 \x20   exits 0\n\n";
    assert!(
        setup.report().starts_with(&format!("{chain}{chain}")),
        "{}",
        setup.report()
    );
    assert_eq!(fixture.names(), ["marker"]);
}

#[test]
fn a_failing_s_own_dir_wins_over_the_fresh_dir_of_examples() {
    let setup = tool("recoveries-fresh-own");
    let (fixture, fixture_path) = marked("recoveries-fresh-own-fixture");
    let (work, work_path) = marked("recoveries-fresh-own-work");
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"]).dir(&work_path)];
    let examples = Examples::new(&setup.script, "").fresh_dir(&fixture_path);
    let recoveries = Recoveries::new(examples, CODES)
        .starts(&starts)
        .unprovoked(OTHERS);

    assert_eq!(problems(&setup, &recoveries), None);
    assert_eq!(work.names(), ["made", "marker"]);
    assert_eq!(fixture.names(), ["marker"]);
}
