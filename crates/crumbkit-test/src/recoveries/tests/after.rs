//! Tests for how the recovery check follows a step that puts a repair
//! before its next step.

use super::{OTHERS, SHOW_7, Setup, UNPROVOKED, failure, listed, problems, recoveries};

/// Returns an `after` step that retries, in JSON.
const AFTER_RETRY: &str = r#"{"kind":"after","repair":"fix it","then":{"kind":"retry"}}"#;

/// Returns an `after` step that runs `tool books list`, in JSON.
const AFTER_LIST: &str =
    r#"{"kind":"after","repair":"fix it","then":{"kind":"run","call":"tool books list"}}"#;

#[test]
fn a_repair_before_a_retry_ends_its_branch_and_counts_as_a_next_step() {
    let show = failure("tool::books::not_found", AFTER_RETRY);
    let setup = Setup::new(
        "recoveries-after-retry",
        &[("books show 7 --json", 1, show.as_str(), "")],
    );

    assert_eq!(problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)), None);
    assert!(
        setup.report().contains("  after a repair, retry\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn the_call_after_a_repair_runs_and_the_report_says_it_follows_a_repair() {
    let show = failure("tool::books::not_found", AFTER_LIST);
    let mut answers = vec![("books show 7 --json", 1, show.as_str(), "")];
    answers.extend(listed());
    let setup = Setup::new("recoveries-after-run", &answers);

    assert_eq!(problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)), None);
    assert!(
        setup
            .report()
            .contains("  after a repair, run tool books list\n    exits 0\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn the_call_after_a_repair_that_the_parser_refuses_fails_the_check() {
    let show = failure("tool::books::not_found", AFTER_LIST);
    let refused = failure("tool::cli::usage", r#"{"kind":"retry"}"#);
    let setup = Setup::new(
        "recoveries-after-refused",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 2, refused.as_str(), ""),
        ],
    );

    let message = problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)).unwrap();

    assert!(message.contains("the parser refuses"), "{message}");
}

#[test]
fn the_call_after_a_repair_that_fails_with_the_same_code_is_no_problem() {
    let show = failure("tool::books::not_found", AFTER_LIST);
    let again = failure("tool::books::not_found", AFTER_LIST);
    let setup = Setup::new(
        "recoveries-after-same-code",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 1, again.as_str(), ""),
        ],
    );

    assert_eq!(problems(&setup, &recoveries(&setup, SHOW_7, OTHERS)), None);
    assert!(
        setup
            .report()
            .contains("    fails with tool::books::not_found, as before the repair\n"),
        "{}",
        setup.report()
    );
}

#[test]
fn the_call_after_a_repair_that_fails_with_another_code_continues_the_chain() {
    let show = failure("tool::books::not_found", AFTER_LIST);
    let list = failure(
        "tool::books::unreadable",
        r#"{"kind":"hint","text":"Fix the file."}"#,
    );
    let setup = Setup::new(
        "recoveries-after-other-code",
        &[
            ("books show 7 --json", 1, show.as_str(), ""),
            ("books list --json", 1, list.as_str(), ""),
        ],
    );

    assert_eq!(
        problems(&setup, &recoveries(&setup, SHOW_7, UNPROVOKED)),
        None
    );
    assert!(
        setup
            .report()
            .contains("  after a repair, run tool books list\n    fails with tool::books::unreadable\n    hint\n"),
        "{}",
        setup.report()
    );
}
