//! Tests for how the recovery check treats a step that names another
//! program's command, which it reports and never runs.

use crumbkit::Tool;
use snapbox::Assert;
use snapbox::assert::Action;

use super::{OTHERS, SHOW_7, Setup, failure, problems, recoveries, tree};
use crate::examples::Examples;
use crate::failures::FailureReader;
use crate::recoveries::{Recoveries, check};
use crate::testing::panic_message;
use crate::{ReadFailure, RecoveryStep};

/// Returns a step that runs `git status`, in JSON.
const GIT_STATUS: &str =
    r#"{"kind":"program","program":"git","args":["status"],"run":"git status"}"#;

#[test]
fn a_program_step_ends_its_branch_and_the_report_names_its_line() {
    let show = failure("tool::books::not_found", GIT_STATUS);
    let setup = Setup::new(
        "recoveries-program-ends-branch",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );

    assert_eq!(problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)), None);
    assert!(
        setup.report().contains("  program git status\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn a_failure_whose_only_step_is_a_program_command_offers_a_next_step() {
    let show = failure("tool::books::not_found", GIT_STATUS);
    let dead_end = failure(
        "tool::books::not_found",
        r#"{"kind":"none","reason":"Nothing to do."}"#,
    );
    let with_program = Setup::new(
        "recoveries-program-next-step",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );
    let without = Setup::new(
        "recoveries-none-next-step",
        &[("books show 7 --json", 1, dead_end.as_str(), "")],
    );

    assert_eq!(
        problems(&with_program, &recoveries(&with_program, SHOW_7, OTHERS)),
        None
    );
    let message = problems(&without, &recoveries(&without, SHOW_7, OTHERS)).unwrap();
    assert!(message.contains("offers no run"), "{message}");
}

/// A reader that gives every failure one program step with an empty command
/// line, as a tool's own reader can.
struct EmptyRun;

impl FailureReader for EmptyRun {
    fn read(&self, _out: &str, _err: &str) -> Vec<ReadFailure> {
        vec![ReadFailure::new(
            "tool::books::not_found".to_owned(),
            vec![RecoveryStep::Program {
                program: "git".to_owned(),
                args: Vec::new(),
                run: String::new(),
            }],
        )]
    }
}

#[test]
fn a_program_step_without_a_command_line_from_a_failure_reader_is_a_problem_and_is_not_reported() {
    let setup = Setup::new(
        "recoveries-program-empty-run",
        &[("books show 7 --json", 1, "", "")],
    );
    let reader = EmptyRun;
    let recoveries = Recoveries::new(
        Examples::new(&setup.script, "").failure_reader(&reader),
        super::CODES,
    )
    .starts(SHOW_7)
    .unprovoked(OTHERS);

    let message = panic_message(|| {
        check(
            &tree(),
            &Tool::new(),
            &recoveries,
            &setup.golden(),
            &Assert::new().action(Action::Overwrite),
        );
    })
    .unwrap();

    assert!(
        message.contains("a program step has no command line in `run`"),
        "{message}"
    );
    assert!(!setup.report().contains("program"), "{}", setup.report());
}
