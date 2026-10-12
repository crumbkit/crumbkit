//! Tests for the recovery check.
//!
//! Each test writes a `/bin/sh` script that stands in for a tool. The script
//! answers each command line that a test names with a fixed exit code,
//! standard output and standard error, and fails any other command line
//! with code 3.

use std::fmt::Write as _;
use std::fs;

use clap::Command;
use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};
use snapbox::Assert;
use snapbox::assert::Action;

use super::{Failing, Recoveries, Unprovoked, check};
use crate::allow::{Allow, Rule};
use crate::examples::Examples;
use crate::temp::TempDir;
use crate::testing::panic_message;

mod form;
mod fresh;
mod values;
mod words;
mod xdg;

/// The code of a book that does not exist.
const NOT_FOUND: Code = Code::new("tool", "books", "not_found");
/// The code of a list of ids that cannot be read.
const UNREADABLE: Code = Code::new("tool", "books", "unreadable");
/// The code of a book file that disappeared, which no command line provokes.
const REMOVED: Code = Code::new("tool", "books", "removed_file");

/// The tool's declared codes.
const CODES: &[&[&Code]] = &[&[&NOT_FOUND, &UNREADABLE, &REMOVED]];

/// The listed code that no command line provokes, for a test whose
/// starts provoke `NOT_FOUND` and `UNREADABLE`.
const UNPROVOKED: &[Unprovoked<'_>] = &[Unprovoked::new(&REMOVED, "Only a full disk provokes it.")];

/// The listed codes that no command line provokes, for a test whose starts
/// provoke only `NOT_FOUND`.
const OTHERS: &[Unprovoked<'_>] = &[
    Unprovoked::new(&UNREADABLE, "Only a broken file provokes it."),
    Unprovoked::new(&REMOVED, "Only a full disk provokes it."),
];

/// The start that fails because book 7 does not exist.
const SHOW_7: &[Failing<'_>] = &[Failing::new(&NOT_FOUND, &["books", "show", "7"])];

/// The tool's command tree: `tool books show` and `tool books list`.
fn tool() -> Command {
    Command::new("tool").subcommand(
        Command::new("books")
            .subcommand(Command::new("show"))
            .subcommand(Command::new("list")),
    )
}

/// One command line that the script answers: its words joined by spaces,
/// its exit code, its standard output and its standard error.
type Answer<'a> = (&'a str, u8, &'a str, &'a str);

/// Returns the line of a failure document with `code` and `recovery`, a
/// step in JSON, such as `{"kind":"retry"}`.
fn failure(code: &str, recovery: &str) -> String {
    format!(
        r#"{{"schema":"crumbkit/failure/v1","code":"{code}","message":"it failed","recovery":{recovery}}}"#
    )
}

/// Returns a step that runs `call`, in JSON.
fn run(call: &str) -> String {
    format!(r#"{{"kind":"run","call":"{call}"}}"#)
}

/// The test's directory, which holds the script and the golden report.
struct Setup {
    dir: TempDir,
    script: String,
}

impl Setup {
    /// Writes a script that gives each of `answers`.
    fn new(label: &str, answers: &[Answer<'_>]) -> Self {
        let dir = TempDir::new(label).unwrap();
        let mut script = String::from("#!/bin/sh\ncase \"$*\" in\n");
        for (line, code, stdout, stderr) in answers {
            writeln!(
                script,
                "  '{line}') printf '%s' '{stdout}'; printf '%s' '{stderr}' >&2; exit {code};;"
            )
            .unwrap();
        }
        script.push_str("  *) echo \"unexpected: $*\" >&2; exit 3;;\nesac\n");
        dir.write_script("tool", &script);
        let script = dir.path().join("tool").display().to_string();
        Self { dir, script }
    }

    /// Returns the path of the golden directory.
    fn golden(&self) -> String {
        self.dir.path().display().to_string()
    }

    /// Returns the golden report that the last overwrite wrote.
    fn report(&self) -> String {
        fs::read_to_string(self.dir.path().join("recoveries.txt")).unwrap()
    }
}

/// Runs the check of `recoveries` on `tool()` with `action`.
fn check_with(setup: &Setup, recoveries: &Recoveries<'_>, action: Action) {
    check(
        &tool(),
        &Guide::new(),
        recoveries,
        &setup.golden(),
        &Assert::new().action(action),
    );
}

/// Returns the panic message of the check of `recoveries` with
/// `SNAPSHOTS=overwrite`, or `None` when it passes.
fn problems(setup: &Setup, recoveries: &Recoveries<'_>) -> Option<String> {
    panic_message(|| check_with(setup, recoveries, Action::Overwrite))
}

/// Returns the settings that run `setup`'s script and check the tool's codes
/// with `starts` and the listed codes `unprovoked`.
fn recoveries<'a>(
    setup: &'a Setup,
    starts: &'a [Failing<'a>],
    unprovoked: &'a [Unprovoked<'a>],
) -> Recoveries<'a> {
    Recoveries::new(Examples::new(&setup.script, ""), Spelling::COLONS, CODES)
        .starts(starts)
        .unprovoked(unprovoked)
}

/// The answers of a chain that ends in a command that succeeds.
fn listed() -> Vec<Answer<'static>> {
    vec![("books list --json", 0, "{}\n", "")]
}

#[test]
fn the_report_names_each_start_its_code_and_each_step_with_its_outcome() {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let mut answers = vec![("books show 7 --json", 1, show.as_str(), "")];
    answers.extend(listed());
    let setup = Setup::new("recoveries-report", &answers);

    check_with(
        &setup,
        &recoveries(&setup, SHOW_7, OTHERS),
        Action::Overwrite,
    );

    assert_eq!(
        setup.report(),
        "tool books show 7\n\
         \x20 fails with tool::books::not_found\n\
         \x20 run tool books list\n\
         \x20   exits 0\n\
         \n\
         codes that no start provokes:\n\
         \x20 tool::books::unreadable: Only a broken file provokes it.\n\
         \x20 tool::books::removed_file: Only a full disk provokes it.\n\
         \n\
         codes whose recoveries the check did not follow, because no declared code spells them: 0\n"
    );
    check_with(&setup, &recoveries(&setup, SHOW_7, OTHERS), Action::Verify);
}

#[test]
fn a_placeholder_takes_its_value_and_an_alternative_starts_its_own_branch() {
    let show = format!(
        r#"{{"schema":"crumbkit/failure/v1","code":"tool::books::unreadable","message":"m","recovery":{{"kind":"hint","text":"Fix the file."}},"alternatives":[{}]}}"#,
        run("tool books show <ID>...")
    );
    let not_found = failure("tool::books::not_found", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-values",
        &[
            ("books show --from ids.txt --json", 1, show.as_str(), ""),
            ("books show 2 --json", 1, not_found.as_str(), ""),
        ],
    );
    let starts = [Failing::new(
        &UNREADABLE,
        &["books", "show", "--from", "ids.txt"],
    )];

    check_with(
        &setup,
        &recoveries(&setup, &starts, UNPROVOKED).values(&[("ID", "2")]),
        Action::Overwrite,
    );

    assert!(
        setup.report().starts_with(
            "tool books show --from ids.txt\n\
             \x20 fails with tool::books::unreadable\n\
             \x20 hint\n\
             \x20 run tool books show <ID>... as tool books show 2\n\
             \x20   fails with tool::books::not_found\n\
             \x20   retry\n\n"
        ),
        "{}",
        setup.report()
    );
}

#[test]
fn a_placeholder_without_a_value_fails_the_check() {
    let show = failure("tool::books::not_found", &run("tool books show <ID>..."));
    let setup = Setup::new(
        "recoveries-no-value",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );

    assert_eq!(
        problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the step `tool books show <ID>...` has the placeholder <ID>, and no value of Failing::values, the fields of the failure or Recoveries::values fills it"
    );
}

#[test]
fn a_lowercase_placeholder_takes_the_value_of_its_name() {
    let show = failure("tool::books::not_found", &run("tool books show <id>..."));
    let setup = Setup::new(
        "recoveries-lowercase",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 2 --json", 0, "{}\n", ""),
        ],
    );

    check_with(
        &setup,
        &recoveries(&setup, SHOW_7, OTHERS).values(&[("id", "2")]),
        Action::Overwrite,
    );

    assert!(
        setup
            .report()
            .contains("  run tool books show <id>... as tool books show 2\n    exits 0\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn a_value_fills_only_the_placeholder_whose_name_matches_its_case() {
    let show = failure("tool::books::not_found", &run("tool books show <id>..."));
    let setup = Setup::new(
        "recoveries-case",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );
    let recoveries = recoveries(&setup, SHOW_7, OTHERS).values(&[("ID", "2")]);

    assert_eq!(
        problems(&setup, &recoveries).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the step `tool books show <id>...` has the placeholder <id>, and no value of Failing::values, the fields of the failure or Recoveries::values fills it"
    );
}

#[test]
fn a_start_that_succeeds_or_fails_with_another_code_fails_the_check() {
    let unreadable = failure("tool::books::unreadable", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-start",
        &[
            ("books show 7 --json", 0, "{}\n", ""),
            ("books show 8 --json", 1, unreadable.as_str(), ""),
        ],
    );
    let starts = [
        Failing::new(&NOT_FOUND, &["books", "show", "7"]),
        Failing::new(&NOT_FOUND, &["books", "show", "8"]),
    ];

    assert_eq!(
        problems(&setup, &recoveries(&setup, &starts, UNPROVOKED)).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the start must fail with tool::books::not_found, and it exits with code 0\n\
         tool books show 8: the start must fail with tool::books::not_found, and it fails with tool::books::unreadable\n\
         tool::books::not_found: no start provokes the declared code, and Recoveries::unprovoked does not list it"
    );
}

#[test]
fn a_step_that_fails_again_with_the_same_code_fails_the_check_unless_an_opt_out_covers_it() {
    let show = failure("tool::books::not_found", &run("tool books show 8"));
    let again = failure("tool::books::not_found", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-same",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 8 --json", 1, again.as_str(), ""),
        ],
    );
    let allow = [Allow::new(
        Rule::SameCode,
        "books show",
        "Each id fails on its own.",
    )];

    let message = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap();
    let allowed = problems(
        &setup,
        &Recoveries::new(
            Examples::new(&setup.script, "").allow(&allow),
            Spelling::COLONS,
            CODES,
        )
        .starts(SHOW_7)
        .unprovoked(OTHERS),
    );

    assert_eq!(
        message,
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the step `tool books show 8` fails again with tool::books::not_found, so an agent that follows the recovery learns nothing new"
    );
    assert_eq!(allowed, None);
}

#[test]
fn a_step_that_the_parser_refuses_or_that_warns_fails_the_check() {
    let show = format!(
        r#"{{"schema":"crumbkit/failure/v1","code":"tool::books::not_found","message":"m","recovery":{},"alternatives":[{}]}}"#,
        run("tool books lst"),
        run("tool books list")
    );
    let usage = failure("crumbkit::cli::usage", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-refused",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books lst --json", 2, usage.as_str(), ""),
            (
                "books list --json",
                0,
                "{}\n",
                "warning: crumbkit left out a recovery\n",
            ),
        ],
    );

    assert_eq!(
        problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the parser refuses the step `tool books lst` with crumbkit::cli::usage\n\
         tool books show 7: the step `tool books list` writes a warning to standard error: warning: crumbkit left out a recovery"
    );
}

#[test]
fn a_chain_stops_at_a_command_line_it_ran_and_at_its_step_limit() {
    let show = failure("tool::books::not_found", &run("tool books show 8"));
    let eight = failure("tool::books::unreadable", &run("tool books show 7"));
    let setup = Setup::new(
        "recoveries-stop",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books show 8 --json", 1, eight.as_str(), ""),
        ],
    );

    check_with(
        &setup,
        &recoveries(&setup, SHOW_7, UNPROVOKED),
        Action::Overwrite,
    );
    let looped = setup.report();
    check_with(
        &setup,
        &recoveries(&setup, SHOW_7, UNPROVOKED).steps(1),
        Action::Overwrite,
    );
    let limited = setup.report();

    assert!(
        looped.starts_with(
            "tool books show 7\n\
             \x20 fails with tool::books::not_found\n\
             \x20 run tool books show 8\n\
             \x20   fails with tool::books::unreadable\n\
             \x20   run tool books show 7\n\
             \x20     not run, because the chain ran it already\n\n"
        ),
        "{looped}"
    );
    assert!(
        limited.contains(
            "\x20   run tool books show 7\n\
             \x20     not run, because the chain reached Recoveries::steps, 1\n"
        ),
        "{limited}"
    );
}

#[test]
fn a_code_from_a_component_the_tool_does_not_declare_is_counted_and_its_recovery_is_not_followed() {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let other = failure("git::network::timed_out", &run("tool books show 8"));
    let setup = Setup::new(
        "recoveries-unknown",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 1, other.as_str(), ""),
        ],
    );

    check_with(
        &setup,
        &recoveries(&setup, SHOW_7, OTHERS),
        Action::Overwrite,
    );

    let report = setup.report();
    assert!(
        report.contains(
            "\x20 run tool books list\n\
             \x20   fails with git::network::timed_out, which no declared code spells\n\n"
        ),
        "{report}"
    );
    assert!(
        report.ends_with("because no declared code spells them: 1\n"),
        "{report}"
    );
}

#[test]
fn an_undeclared_code_from_a_declared_component_fails_the_check() {
    let show = failure("tool::books::not_found", &run("tool books list"));
    let lost = failure("tool::books::lost", r#"{"kind":"none"}"#);
    let setup = Setup::new(
        "recoveries-undeclared",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 1, lost.as_str(), ""),
        ],
    );

    let problems = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap();

    assert!(
        problems.contains(
            "tool books show 7: the step `tool books list` fails with tool::books::lost, \
             whose component tool is a declared component, and no declared code spells it"
        ),
        "{problems}"
    );
    let report = setup.report();
    assert!(
        report.ends_with("because no declared code spells them: 0\n"),
        "{report}"
    );
}

#[test]
fn every_declared_code_is_provoked_or_listed_and_each_listed_code_is_declared_and_unprovoked() {
    const STRAY: Code = Code::new("tool", "stray", "gone");
    let show = failure("tool::books::not_found", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-codes",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );
    let unprovoked = [
        Unprovoked::new(&NOT_FOUND, "A missing book needs a full disk."),
        Unprovoked::new(&STRAY, "Nothing provokes it."),
    ];
    let starts = [
        Failing::new(&NOT_FOUND, &["books", "show", "7"]),
        Failing::new(&STRAY, &["books", "show", "9"]),
    ];

    let message = problems(
        &setup,
        &Recoveries::new(Examples::new(&setup.script, ""), Spelling::COLONS, CODES)
            .starts(&starts)
            .unprovoked(&unprovoked),
    )
    .unwrap();

    assert_eq!(
        message,
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 9: the start's code tool::stray::gone is not a declared code\n\
         tool::books::unreadable: no start provokes the declared code, and Recoveries::unprovoked does not list it\n\
         tool::books::removed_file: no start provokes the declared code, and Recoveries::unprovoked does not list it\n\
         tool::books::not_found: Recoveries::unprovoked lists the code, and a start provokes it\n\
         tool::stray::gone: Recoveries::unprovoked lists the code, and it is not a declared code"
    );
}

#[test]
fn an_opt_out_skips_the_starts_below_its_path_and_a_stale_one_fails_the_check() {
    let setup = Setup::new("recoveries-opt-out", &[]);
    let allow = [
        Allow::new(Rule::Recoveries, "books show", "Each run needs a server."),
        Allow::new(Rule::Recoveries, "books list", "Each run needs a server."),
    ];
    let unprovoked = [
        Unprovoked::new(&NOT_FOUND, "Only a server provokes it."),
        Unprovoked::new(&UNREADABLE, "Only a server provokes it."),
        Unprovoked::new(&REMOVED, "Only a server provokes it."),
    ];

    let message = problems(
        &setup,
        &Recoveries::new(
            Examples::new(&setup.script, "").allow(&allow),
            Spelling::COLONS,
            CODES,
        )
        .starts(SHOW_7)
        .unprovoked(&unprovoked),
    )
    .unwrap();

    assert_eq!(
        message,
        "the tool's recoveries break crumbkit's recovery rules:\n\
         the opt-out from `recoveries/follow` for `tool books list` excuses no problem, so it is stale"
    );
}

#[test]
fn a_call_that_does_not_start_with_the_tool_s_name_fails_the_check() {
    let show = failure("tool::books::not_found", &run("other books list"));
    let setup = Setup::new(
        "recoveries-name",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );

    assert_eq!(
        problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap(),
        "the tool's recoveries break crumbkit's recovery rules:\n\
         tool books show 7: the step `other books list` must start with the tool's name, tool"
    );
}

#[test]
fn a_batch_s_first_failure_starts_the_chain() {
    let batch = format!(
        r#"{{"schema":"tool/books-show/v1","data":[],"failures":[{{"code":"tool::books::not_found","message":"m","recovery":{}}}]}}"#,
        run("tool books list")
    );
    let mut answers = vec![("books show 7 --json", 1, batch.as_str(), "")];
    answers.extend(listed());
    let setup = Setup::new("recoveries-batch", &answers);

    check_with(
        &setup,
        &recoveries(&setup, SHOW_7, OTHERS),
        Action::Overwrite,
    );

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
