//! Tests for the environment variables of a start, which every run of its
//! chain sets.

use snapbox::assert::Action;

use super::{CODES, NOT_FOUND, OTHERS, Setup, check_with, failure, run};
use crate::examples::Examples;
use crate::recoveries::{Failing, Recoveries};
use crate::temp::TempDir;

#[test]
fn every_run_of_a_chain_sets_the_variables_of_its_start_and_the_report_names_them() {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let dir = TempDir::new("recoveries-env").unwrap();
    // Each run answers only with `DEPTH=1` and without `HOME`.
    dir.write_script(
        "tool",
        &format!(
            r#"#!/bin/sh
if [ "$DEPTH" != 1 ] || [ -n "${{HOME+x}}" ]; then exit 3; fi
case "$*" in
  'books show 7 --json') printf '%s' '{show}'; exit 1;;
  'books list --json') echo '{{}}'; exit 0;;
  *) exit 3;;
esac
"#
        ),
    );
    let script = dir.path().join("tool").display().to_string();
    let setup = Setup { dir, script };
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"])
        .env(&[("DEPTH", Some("1")), ("HOME", None)])];
    let examples = Examples::new(&setup.script, "").env(&[("HOME", "/home")]);
    let recoveries = Recoveries::new(examples, CODES)
        .starts(&starts)
        .unprovoked(OTHERS);

    check_with(&setup, &recoveries, Action::Overwrite);

    assert!(
        setup.report().starts_with(
            "tool books show 7 (with DEPTH=1, HOME removed)\n\
             \x20 fails with tool::books::not_found\n\
             \x20 run tool books list\n\
             \x20   exits 0\n\n"
        ),
        "{}",
        setup.report()
    );
}
