//! Tests for how the recovery check treats a step that names another
//! program's command, which it reports and never runs.

use super::{OTHERS, SHOW_7, Setup, failure, problems, recoveries};

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
