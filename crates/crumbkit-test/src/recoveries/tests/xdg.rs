//! Tests for the XDG fixtures that the recovery check copies into the XDG
//! directories of each chain.

use snapbox::assert::Action;

use super::{CODES, NOT_FOUND, OTHERS, Setup, check_with, failure, problems, run};
use crate::examples::Examples;
use crate::recoveries::{Failing, Recoveries};
use crate::temp::TempDir;

/// Returns a tool whose `books show 7` fails with `NOT_FOUND` and offers
/// `tool books list` only when the data directory holds `tool/b` and not
/// `tool/a`, and whose `books list` succeeds only when it holds `tool/b`.
fn tool(label: &str) -> Setup {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let dir = TempDir::new(label).unwrap();
    dir.write_script(
        "tool",
        &format!(
            r#"#!/bin/sh
data="$XDG_DATA_HOME/tool"
case "$*" in
  'books show 7 --json') if [ -f "$data/b" ] && [ ! -f "$data/a" ]; then printf '%s' '{show}'; exit 1; fi; exit 3;;
  'books list --json') if [ -f "$data/b" ]; then echo '{{}}'; exit 0; fi; exit 3;;
  *) exit 3;;
esac
"#
        ),
    );
    let script = dir.path().join("tool").display().to_string();
    Setup { dir, script }
}

/// Returns a fixture whose data directory holds the file `tool/<name>`.
fn fixture(label: &str, name: &str) -> (TempDir, String) {
    let fixture = TempDir::new(label).unwrap();
    fixture.create_dir("data/tool").unwrap();
    fixture.write(&format!("data/tool/{name}"), "");
    let path = fixture.path().display().to_string();
    (fixture, path)
}

#[test]
fn a_start_s_xdg_fixture_is_in_its_chain_s_directories_before_it_runs() {
    let setup = tool("recoveries-xdg");
    let (_b, b) = fixture("recoveries-xdg-b", "b");
    let starts = [Failing::new(&NOT_FOUND, &["books", "show", "7"]).xdg(&b)];
    let recoveries = Recoveries::new(Examples::new(&setup.script, ""), CODES)
        .starts(&starts)
        .unprovoked(OTHERS);

    check_with(&setup, &recoveries, Action::Overwrite);

    assert!(
        setup.report().starts_with(
            "tool books show 7\n\
             \x20 fails with tool::books::not_found\n\
             \x20 run tool books list\n\
             \x20   exits 0\n\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_failing_s_own_xdg_fixture_wins_over_the_examples_fixture() {
    let setup = tool("recoveries-xdg-wins");
    let (_a, a) = fixture("recoveries-xdg-wins-a", "a");
    let (_b, b) = fixture("recoveries-xdg-wins-b", "b");
    let examples = Examples::new(&setup.script, "").xdg(&a);
    let own = [Failing::new(&NOT_FOUND, &["books", "show", "7"]).xdg(&b)];
    let shared = [Failing::new(&NOT_FOUND, &["books", "show", "7"])];
    let recoveries = |starts| {
        Recoveries::new(examples, CODES)
            .starts(starts)
            .unprovoked(OTHERS)
    };

    let with_own = problems(&setup, &recoveries(&own));
    let without = problems(&setup, &recoveries(&shared));

    assert_eq!(with_own, None);
    assert_eq!(
        without.unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the start ends with exit status: 3 and prints no failure that the check can read\n\
         tool::books::not_found: no start provokes the declared code, and Recoveries::unprovoked does not list it"
    );
}
