//! Tests for `RuntimeFailing`, which builds a failing command line from
//! data that a test reads at run time.
//!
//! Each test writes a `/bin/sh` script that stands in for a tool, as the
//! recovery check's tests do.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt as _;
use std::path::PathBuf;

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

/// Returns the words of `line`, split at spaces, as owned strings.
fn words(line: &str) -> Vec<String> {
    line.split(' ').map(str::to_owned).collect()
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
    let owned = [RuntimeFailing::new(&NOT_FOUND, words("lists check"))
        .dir(work.path().to_path_buf())
        .stdin("everything".to_owned())
        .xdg(xdg.path().to_path_buf())
        .values(vec![("NAME".to_owned(), "present".to_owned())])
        .env(vec![("DEPTH".to_owned(), Some("1".to_owned()))])
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
        RuntimeFailing::new(&UNREADABLE, words(&format!("lists show {name}"))).untwinned(reason)
    };

    assert_eq!(allow.line(&tree()), "tool lists show broken");
    assert_eq!(
        allow.reason(),
        "The list broken is broken, and no word of the line names that."
    );
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
    let show = |name: &str| words(&format!("lists show {name}"));
    let refused = |code: &'static Code, name: &str| {
        RuntimeFailing::new(code, show(name)).dir(work.path().to_path_buf())
    };
    let pairs = [refused(&NOT_FOUND, &missing).twin(show(&present))];
    let allow = [refused(&UNREADABLE, &broken).untwinned(format!(
        "The list {broken} is broken, and no word of the line names that."
    ))];
    let starts = [
        refused(&NOT_FOUND, &missing)
            .values(vec![("NAME".to_owned(), present.clone())])
            .into_failing(),
        refused(&UNREADABLE, &broken).into_failing(),
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

    let failing = RuntimeFailing::new(&NOT_FOUND, words("lists check"))
        .dir(path.clone())
        .xdg(path.clone())
        .into_failing();

    let Some(Dir::Fixed(dir)) = failing.work_dir(None) else {
        panic!("the failing has no directory of its own");
    };
    assert_eq!(dir.path(), path);
    assert_eq!(failing.fixture(None), Some(path.as_path()));
}
