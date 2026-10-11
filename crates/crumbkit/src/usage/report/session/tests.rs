use super::{Found, Session};
use crate::usage::report::entry::Entry;
use crate::usage::report::{Finding, Kind};

/// Returns the findings of one session's `entries`, added in order.
fn findings(entries: &[Entry]) -> Vec<Finding> {
    let mut session = Session::default();
    let mut found = Found::default();
    for entry in entries {
        session.add(entry, &mut found);
    }
    session.end(&mut found);
    found.into_findings().collect()
}

/// Returns the finding of `kind` for `command` with `runs` and no code.
fn finding(kind: Kind, command: &str, runs: u64) -> Finding {
    Finding {
        kind,
        command: command.to_owned(),
        runs,
        code: None,
    }
}

/// Returns a run of `books show` with one id that ended `at` seconds after
/// the Unix epoch.
fn one_id(at: u64) -> Entry {
    let mut entry = Entry::run("s", at, "books show");
    entry.ids = Some(1);
    entry
}

/// Returns a help page of `command` read `at` seconds after the Unix epoch.
fn help(at: u64, command: &str) -> Entry {
    let mut entry = Entry::run("s", at, command);
    entry.help = true;
    entry
}

/// Returns a run of `books show` that failed with `shelf::books::not_found`
/// `at` seconds after the Unix epoch.
fn failed(at: u64) -> Entry {
    let mut entry = Entry::run("s", at, "books show");
    entry.exit = Some(1);
    entry.code = Some("shelf::books::not_found".to_owned());
    entry
}

/// Returns a run of `books list` that ended `at` seconds after the Unix
/// epoch and typed `flags`.
fn list(at: u64, flags: &[&str]) -> Entry {
    let mut entry = Entry::run("s", at, "books list");
    entry.flags = flags.iter().map(|flag| (*flag).to_owned()).collect();
    entry
}

/// Returns a run of `books list` that showed a cut-short listing `at`
/// seconds after the Unix epoch.
fn cut_short(at: u64) -> Entry {
    let mut entry = list(at, &[]);
    entry.truncated = true;
    entry
}

#[test]
fn three_single_id_runs_of_a_command_within_two_minutes_are_one_id_at_a_time() {
    let entries = [one_id(1000), one_id(1060), one_id(1120)];

    assert_eq!(
        findings(&entries),
        [finding(Kind::OneIdAtATime, "books show", 3)]
    );
}

#[test]
fn two_single_id_runs_are_not_one_id_at_a_time() {
    assert_eq!(findings(&[one_id(1000), one_id(1001)]), []);
}

#[test]
fn three_single_id_runs_over_more_than_two_minutes_are_not_one_id_at_a_time() {
    let entries = [one_id(1000), one_id(1060), one_id(1121)];

    assert_eq!(findings(&entries), []);
}

#[test]
fn each_single_id_run_of_a_burst_counts_once() {
    let entries = [
        one_id(1000),
        one_id(1010),
        one_id(1020),
        one_id(1030),
        one_id(1200),
        one_id(1210),
    ];

    assert_eq!(
        findings(&entries),
        [finding(Kind::OneIdAtATime, "books show", 4)]
    );
}

#[test]
fn a_run_with_two_ids_is_not_counted_as_one_id_at_a_time() {
    let mut two = one_id(1060);
    two.ids = Some(2);

    assert_eq!(findings(&[one_id(1000), two, one_id(1120)]), []);
}

#[test]
fn the_same_command_line_within_60_seconds_is_an_identical_repeat() {
    let first = list(1000, &[]);
    let mut again = list(1059, &[]);
    again.args.clone_from(&first.args);

    assert_eq!(
        findings(&[first, again]),
        [finding(Kind::IdenticalRepeat, "books list", 1)]
    );
}

#[test]
fn the_same_command_line_after_61_seconds_is_not_a_repeat() {
    let first = list(1000, &[]);
    let mut again = list(1061, &[]);
    again.args.clone_from(&first.args);

    assert_eq!(findings(&[first, again]), []);
}

#[test]
fn three_help_pages_before_the_first_success_are_a_help_loop_under_the_last_page() {
    let entries = [
        help(1000, ""),
        help(1010, "books"),
        help(1020, "books list"),
        list(1030, &[]),
        help(1040, "books show"),
    ];

    assert_eq!(
        findings(&entries),
        [finding(Kind::HelpLoop, "books list", 3)]
    );
}

#[test]
fn two_help_pages_before_the_first_success_are_not_a_help_loop() {
    let entries = [help(1000, ""), help(1010, "books list"), list(1020, &[])];

    assert_eq!(findings(&entries), []);
}

#[test]
fn three_help_pages_in_a_session_that_never_succeeds_are_a_help_loop() {
    let mut refused = list(1030, &[]);
    refused.exit = Some(2);
    let entries = [
        help(1000, ""),
        help(1010, "books"),
        help(1020, "books show"),
        refused,
    ];

    assert_eq!(
        findings(&entries),
        [finding(Kind::HelpLoop, "books show", 3)]
    );
}

#[test]
fn a_failure_followed_by_the_same_command_within_60_seconds_is_a_retry_under_its_code() {
    let entries = [failed(1000), Entry::run("s", 1059, "books show")];

    assert_eq!(
        findings(&entries),
        [Finding {
            kind: Kind::RetryAfterFailure,
            command: "books show".to_owned(),
            runs: 1,
            code: Some("shelf::books::not_found".to_owned()),
        }]
    );
}

#[test]
fn the_same_command_61_seconds_after_a_failure_is_not_a_retry() {
    let entries = [failed(1000), Entry::run("s", 1061, "books show")];

    assert_eq!(findings(&entries), []);
}

#[test]
fn a_retry_counts_only_the_run_right_after_the_failure() {
    let entries = [
        failed(1000),
        Entry::run("s", 1010, "books show"),
        Entry::run("s", 1020, "books show"),
    ];

    assert_eq!(findings(&entries).len(), 1);
}

#[test]
fn a_cut_short_listing_followed_by_all_or_limit_within_60_seconds_is_a_short_page() {
    let entries = [
        cut_short(1000),
        list(1059, &["--all"]),
        cut_short(2000),
        list(2010, &["--limit"]),
    ];

    assert_eq!(
        findings(&entries),
        [finding(Kind::ShortPage, "books list", 2)]
    );
}

#[test]
fn the_next_page_61_seconds_later_or_without_all_or_limit_is_not_a_short_page() {
    let entries = [
        cut_short(1000),
        list(1061, &["--all"]),
        cut_short(2000),
        list(2010, &["--long"]),
    ];

    assert_eq!(findings(&entries), []);
}
