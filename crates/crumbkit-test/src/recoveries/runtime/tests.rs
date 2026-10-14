//! Tests for `RuntimeFailing`, which builds a failing command line from
//! data that a test reads at run time.
//!
//! Each test writes a `/bin/sh` script that stands in for a tool, as the
//! recovery check's tests do.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};

use clap::{Arg, Command};
use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};
use snapbox::Assert;
use snapbox::assert::Action;

use super::RuntimeFailing;
use crate::examples::Examples;
use crate::recoveries::{Failing, Recoveries, Unprovoked, check};
use crate::runs::Dir;
use crate::temp::TempDir;
use crate::testing::panic_message;
use crate::twins::{Twins, check_twins};

/// The code of a list that does not exist.
const NOT_FOUND: Code = Code::new("tool", "lists", "not_found");
/// The code of a list file that holds no list.
const UNREADABLE: Code = Code::new("tool", "lists", "unreadable");

/// The tool's declared codes.
const CODES: &[&[&Code]] = &[&[&NOT_FOUND, &UNREADABLE]];

/// The script's answers, with or without `--json` at the end:
///
/// - `lists show NAME` fails with `NOT_FOUND` and offers
///   `tool lists show <NAME>` when the working directory has no file
///   `NAME`, fails with `UNREADABLE` and a hint when the file holds
///   `broken`, and exits with code 0 otherwise;
/// - `lists check` fails with `NOT_FOUND` and offers `tool lists show
///   <NAME>` only when the working directory holds `present`, the XDG
///   config directory holds `tool/strict`, `DEPTH` is `1` and standard
///   input is `everything`.
///
/// Any other command line fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
line="$*"
line="${line% --json}"
input=$(cat)
not_found='{"schema":"crumbkit/failure/v1","code":"tool::lists::not_found","message":"m","recovery":{"kind":"run","call":"tool lists show <NAME>"}}'
case "$line" in
  'lists show '*)
    name="${line#lists show }"
    if [ ! -f "$name" ]; then printf '%s\n' "$not_found"; exit 1; fi
    if [ "$(cat "$name")" = broken ]; then
      printf '%s\n' '{"schema":"crumbkit/failure/v1","code":"tool::lists::unreadable","message":"m","recovery":{"kind":"hint","text":"Fix the list."}}'
      exit 1
    fi
    echo shown; exit 0;;
  'lists check')
    if [ -f present ] && [ -f "$XDG_CONFIG_HOME/tool/strict" ] && [ "$DEPTH" = 1 ] && [ "$input" = everything ]; then
      printf '%s\n' "$not_found"; exit 1
    fi
    exit 3;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// Returns the tool's command tree: `tool lists show NAME` and
/// `tool lists check`.
fn tree() -> Command {
    Command::new("tool").subcommand(
        Command::new("lists")
            .subcommand(Command::new("show").arg(Arg::new("name")))
            .subcommand(Command::new("check")),
    )
}

/// Writes the script and returns its directory and path.
fn tool(label: &str) -> (TempDir, String) {
    let dir = TempDir::new(label).unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    (dir, path)
}

/// Returns a working directory that holds the list `present` and the
/// broken list `broken`.
fn lists(label: &str) -> TempDir {
    let dir = TempDir::new(label).unwrap();
    dir.write("present", "a list");
    dir.write("broken", "broken");
    dir
}

/// Returns an XDG fixture whose config directory holds `tool/strict`.
fn strict(label: &str) -> TempDir {
    let fixture = TempDir::new(label).unwrap();
    fixture.create_dir("config/tool").unwrap();
    fixture.write("config/tool/strict", "");
    fixture
}

/// Runs the recovery check of `recoveries` with `SNAPSHOTS=overwrite`,
/// requires it to pass and returns the report that it wrote.
fn report(recoveries: &Recoveries<'_>, label: &str) -> String {
    let golden = TempDir::new(label).unwrap();
    let dir = golden.path().display().to_string();
    let problems = panic_message(|| {
        check(
            &tree(),
            &Guide::new(),
            recoveries,
            &dir,
            &Assert::new().action(Action::Overwrite),
        );
    });
    assert_eq!(problems, None);
    fs::read_to_string(golden.path().join("recoveries.txt")).unwrap()
}

#[test]
fn a_runtime_failing_built_from_owned_strings_runs_as_a_borrowed_failing_does() {
    let (_tool, script) = tool("runtime-failing");
    let work = lists("runtime-failing-work");
    let xdg = strict("runtime-failing-xdg");
    let work_text = work.path().display().to_string();
    let xdg_text = xdg.path().display().to_string();
    let unprovoked = [Unprovoked::new(
        &UNREADABLE,
        "Only a broken list provokes it.",
    )];
    let examples = Examples::new(&script, "");
    let borrowed = [Failing::new(&NOT_FOUND, &["lists", "check"])
        .dir(&work_text)
        .stdin("everything")
        .xdg(&xdg_text)
        .values(&[("NAME", "present")])
        .env(&[("DEPTH", Some("1"))])];
    let owned = [RuntimeFailing::new(NOT_FOUND, ["lists", "check"])
        .dir(work.path())
        .stdin(String::from("everything"))
        .xdg(xdg.path())
        .values([("NAME", String::from("present"))])
        .env("DEPTH", "1")
        .into_failing()];
    let recoveries = |starts| {
        Recoveries::new(examples, Spelling::COLONS, CODES)
            .starts(starts)
            .unprovoked(&unprovoked)
    };

    let borrowed_report = report(&recoveries(&borrowed), "runtime-failing-borrowed");
    let owned_report = report(&recoveries(&owned), "runtime-failing-owned");

    assert!(
        owned_report.starts_with(
            "tool lists check (with DEPTH=1)\n\
             \x20 fails with tool::lists::not_found\n\
             \x20 run tool lists show <NAME> as tool lists show present\n\
             \x20   exits 0\n"
        ),
        "{owned_report}"
    );
    assert_eq!(owned_report, borrowed_report);
}

#[test]
fn a_runtime_failing_gives_an_untwinned_opt_out_a_static_line() {
    let allow = {
        let name = String::from("broken");
        let reason = format!("The list {name} is broken, and no word of the line names that.");
        RuntimeFailing::new(UNREADABLE, ["lists", "show", name.as_str()]).untwinned(reason)
    };

    assert_eq!(allow.line(&tree()), "tool lists show broken");
    assert_eq!(
        allow.reason(),
        "The list broken is broken, and no word of the line names that."
    );
}

#[test]
fn an_untwinned_opt_out_keeps_a_constant_reason_without_copying_it() {
    const REASON: &str = "The list is broken, and no word of the line names that.";

    let allow = RuntimeFailing::new(UNREADABLE, ["lists", "show", "broken"]).untwinned(REASON);

    assert!(std::ptr::eq(allow.reason(), REASON));
}

#[test]
fn a_runtime_failing_takes_its_code_by_value() {
    let code = Code::new("tool", "lists", "not_found");

    let failing = RuntimeFailing::new(code, ["lists", "check"]).into_failing();

    assert_eq!(*failing.code(), NOT_FOUND);
    assert_eq!(failing.words(), ["lists", "check"]);
}

#[test]
fn a_line_keeps_the_directory_input_fixture_values_and_variables_and_replaces_the_code_and_words() {
    let place = RuntimeFailing::new(NOT_FOUND, ["lists", "check"])
        .dir("work")
        .stdin("everything")
        .xdg("fixture")
        .values([("NAME", "present")])
        .env("DEPTH", "1")
        .env_remove("HOME");

    let failing = place
        .line(UNREADABLE, ["lists", "show", "broken"])
        .into_failing();

    assert_eq!(*failing.code(), UNREADABLE);
    assert_eq!(failing.words(), ["lists", "show", "broken"]);
    let Some(Dir::Fixed(dir)) = failing.work_dir(None) else {
        panic!("the line has no directory of its own");
    };
    assert_eq!(dir.path(), Path::new("work"));
    assert_eq!(failing.stdin_bytes(), Some("everything".as_bytes()));
    assert_eq!(failing.fixture(None), Some(Path::new("fixture")));
    assert_eq!(failing.own_values(), [("NAME", "present")]);
    assert_eq!(failing.own_env(), [("DEPTH", Some("1")), ("HOME", None)]);
    assert_eq!(*place.into_failing().code(), NOT_FOUND);
}

#[test]
fn an_optional_xdg_fixture_sets_the_fixture_when_it_is_some_and_none_when_it_is_none() {
    let fixture: Option<PathBuf> = Some(PathBuf::from("fixture"));
    let line = RuntimeFailing::new(NOT_FOUND, ["lists", "check"]);

    let with = line.clone().xdg_option(fixture).into_failing();
    let without = line.xdg("fixture").xdg_option(None).into_failing();

    assert_eq!(with.fixture(None), Some(Path::new("fixture")));
    assert_eq!(without.fixture(None), None);
}

#[test]
fn a_test_builds_a_twin_a_recovery_start_and_an_untwinned_opt_out_from_a_directory_it_reads() {
    let (_tool, script) = tool("runtime-scan");
    let work = lists("runtime-scan-work");
    let mut present = String::new();
    let mut broken = String::new();
    for entry in fs::read_dir(work.path()).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().into_string().unwrap();
        if fs::read_to_string(entry.path()).unwrap() == "broken" {
            broken = name;
        } else {
            present = name;
        }
    }
    let missing = format!("{present}-gone");
    let place = RuntimeFailing::new(NOT_FOUND, ["lists"]).dir(work.path());
    let show = |name: &str| ["lists".to_owned(), "show".to_owned(), name.to_owned()];
    let pairs = [place.line(NOT_FOUND, show(&missing)).twin(show(&present))];
    let allow = [place.line(UNREADABLE, show(&broken)).untwinned(format!(
        "The list {broken} is broken, and no word of the line names that."
    ))];
    let starts = [
        place
            .line(NOT_FOUND, show(&missing))
            .values([("NAME", present.clone())])
            .into_failing(),
        place.line(UNREADABLE, show(&broken)).into_failing(),
    ];
    let examples = Examples::new(&script, "").allow(&allow);
    let twins = Twins::new(examples, Spelling::COLONS, CODES).pairs(&pairs);
    let recoveries = Recoveries::new(examples, Spelling::COLONS, CODES).starts(&starts);

    let twin_problems = panic_message(|| check_twins(&tree(), &Guide::new(), &twins));
    let report = report(&recoveries, "runtime-scan-golden");

    assert_eq!(twin_problems, None);
    assert!(
        report.starts_with(&format!(
            "tool lists show {missing}\n\
             \x20 fails with tool::lists::not_found\n\
             \x20 run tool lists show <NAME> as tool lists show {present}\n\
             \x20   exits 0\n"
        )),
        "{report}"
    );
}

#[test]
fn a_runtime_failing_keeps_a_directory_and_a_fixture_whose_paths_are_not_utf_8() {
    let path = PathBuf::from(OsStr::from_bytes(b"lists-\xff"));

    let failing = RuntimeFailing::new(NOT_FOUND, ["lists", "check"])
        .dir(path.clone())
        .xdg(path.clone())
        .into_failing();

    let Some(Dir::Fixed(dir)) = failing.work_dir(None) else {
        panic!("the failing has no directory of its own");
    };
    assert_eq!(dir.path(), path);
    assert_eq!(failing.fixture(None), Some(path.as_path()));
}

#[test]
fn a_place_made_with_at_gives_each_line_built_from_it_its_directory() {
    let place = RuntimeFailing::at("work").env("DEPTH", "1");

    let failing = place.line(NOT_FOUND, ["lists", "check"]).into_failing();

    let Some(Dir::Fixed(dir)) = failing.work_dir(None) else {
        panic!("the line has no directory of its own");
    };
    assert_eq!(dir.path(), Path::new("work"));
    assert_eq!(*failing.code(), NOT_FOUND);
    assert_eq!(failing.words(), ["lists", "check"]);
    assert_eq!(failing.own_env(), [("DEPTH", Some("1"))]);
}

#[test]
fn get_dir_returns_the_directory_that_at_or_dir_set_and_none_without_one() {
    let at = RuntimeFailing::at("work");
    let dir = RuntimeFailing::new(NOT_FOUND, ["lists"]).dir("other");
    let none = RuntimeFailing::new(NOT_FOUND, ["lists"]);

    assert_eq!(at.get_dir(), Some(Path::new("work")));
    assert_eq!(dir.get_dir(), Some(Path::new("other")));
    assert_eq!(none.get_dir(), None);
}

/// A script whose `lists check` fails with `NOT_FOUND` when `DEPTH` is `1`
/// and exits with code 0 otherwise, with or without `--json` at the end.
const DEPTH_SCRIPT: &str = r#"#!/bin/sh
line="$*"
case "${line% --json}" in
  'lists check')
    if [ "$DEPTH" = 1 ]; then
      printf '%s\n' '{"schema":"crumbkit/failure/v1","code":"tool::lists::not_found","message":"m","recovery":{"kind":"retry"}}'
      exit 1
    fi
    exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

#[test]
fn a_variable_twin_takes_a_variable_name_that_the_test_builds_at_run_time() {
    let dir = TempDir::new("runtime-variable").unwrap();
    dir.write_script("tool", DEPTH_SCRIPT);
    let script = dir.path().join("tool").display().to_string();
    let name = ["DEP", "TH"].concat();
    let pairs = [RuntimeFailing::new(NOT_FOUND, ["lists", "check"])
        .env(name.clone(), "1")
        .variable_twin(name)];
    let unprovoked = [Unprovoked::new(
        &UNREADABLE,
        "Only a broken list provokes it.",
    )];
    let twins = Twins::new(Examples::new(&script, ""), Spelling::COLONS, CODES)
        .pairs(&pairs)
        .unprovoked(&unprovoked);

    let problems = panic_message(|| check_twins(&tree(), &Guide::new(), &twins));

    assert_eq!(problems, None);
}

#[test]
fn recoveries_built_from_owned_codes_report_as_recoveries_built_from_code_lists_do() {
    let (_tool, script) = tool("runtime-owned-codes");
    let work = lists("runtime-owned-codes-work");
    let work_text = work.path().display().to_string();
    let examples = Examples::new(&script, "");
    let starts = [Failing::new(&NOT_FOUND, &["lists", "show", "gone"])
        .dir(&work_text)
        .values(&[("NAME", "present")])];
    let reason = "Only a broken list provokes it.";
    let listed = [Unprovoked::new(&UNREADABLE, reason)];
    let codes: Vec<Code> = vec![NOT_FOUND, UNREADABLE];
    let owned = [Unprovoked::from_code(codes[1], reason)];
    let from_lists = Recoveries::new(examples, Spelling::COLONS, CODES)
        .starts(&starts)
        .unprovoked(&listed);
    let from_codes = Recoveries::from_codes(examples, Spelling::COLONS, &codes)
        .starts(&starts)
        .unprovoked(&owned);

    let lists_report = report(&from_lists, "runtime-owned-codes-lists");
    let codes_report = report(&from_codes, "runtime-owned-codes-codes");

    assert!(
        codes_report.contains("tool::lists::unreadable: Only a broken list provokes it."),
        "{codes_report}"
    );
    assert_eq!(codes_report, lists_report);
}
