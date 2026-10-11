use super::{Tally, p95};
use crate::usage::report::entry::Entry;
use crate::usage::report::{Count, Figures, Report};

/// Returns the report of `entries`, added in order.
fn report(entries: &[Entry]) -> Report {
    let mut tally = Tally::default();
    for entry in entries {
        tally.add(entry);
    }
    tally.report()
}

/// Returns the figures of `command` in `report`.
fn figures<'a>(report: &'a Report, command: &str) -> &'a Figures {
    report
        .commands
        .iter()
        .find(|figures| figures.command == command)
        .unwrap()
}

/// Returns a count of `runs` for `name`.
fn count(name: &str, runs: u64) -> Count {
    Count {
        name: name.to_owned(),
        runs,
    }
}

#[test]
fn the_report_counts_runs_sessions_and_the_runs_of_each_caller_the_most_first() {
    let mut agent = Entry::run("a", 1, "books list");
    agent.caller = "claude-code".to_owned();
    let entries = [
        Entry::run("a", 1, "books list"),
        agent,
        Entry::run("b", 2, "books show"),
        Entry::run("c", 3, ""),
    ];

    let report = report(&entries);

    assert_eq!(report.runs, 4);
    assert_eq!(report.sessions, 3);
    assert_eq!(
        report.callers,
        [count("person", 3), count("claude-code", 1)]
    );
}

#[test]
fn each_command_path_has_its_own_figures_and_the_root_has_the_empty_path() {
    let entries = [
        Entry::run("a", 1, "books show"),
        Entry::run("a", 2, "books list"),
        Entry::run("a", 3, ""),
    ];

    let report = report(&entries);

    let paths: Vec<&str> = report
        .commands
        .iter()
        .map(|figures| figures.command.as_str())
        .collect();
    assert_eq!(paths, ["", "books list", "books show"]);
}

#[test]
fn a_help_page_counts_towards_its_command_s_help_and_not_its_runs_or_times() {
    let mut help = Entry::run("a", 1, "books list");
    help.help = true;
    help.ms = 5_000;
    help.out_bytes = None;

    let report = report(&[help, Entry::run("a", 2, "books list")]);

    let figures = figures(&report, "books list");
    assert_eq!(figures.help, 1);
    assert_eq!(figures.runs, 1);
    assert_eq!(figures.p95_ms, Some(10));
}

#[test]
fn refused_runs_count_the_flags_that_clap_suggested_the_most_suggested_first() {
    let refused = |at, nearest: Option<&str>| {
        let mut entry = Entry::run("a", at, "books show");
        entry.exit = Some(2);
        entry.code = Some("crumbkit::cli::usage".to_owned());
        entry.nearest = nearest.map(str::to_owned);
        entry.out_bytes = None;
        entry
    };
    let entries = [
        refused(1, Some("--long")),
        refused(2, Some("--from")),
        refused(3, Some("--from")),
        refused(4, None),
        Entry::run("a", 5, "books show"),
    ];

    let report = report(&entries);

    let figures = figures(&report, "books show");
    assert_eq!(figures.runs, 5);
    assert_eq!(figures.refused, 4);
    assert_eq!(figures.nearest, [count("--from", 2), count("--long", 1)]);
}

#[test]
fn a_run_that_exited_with_1_counts_as_failed_unless_a_signal_ended_it() {
    let failed = |at, interrupted| {
        let mut entry = Entry::run("a", at, "books show");
        entry.exit = Some(1);
        entry.code = Some("shelf::books::not_found".to_owned());
        entry.interrupted = interrupted;
        entry
    };

    let report = report(&[failed(1, false), failed(2, true)]);

    assert_eq!(figures(&report, "books show").failed, 1);
}

#[test]
fn the_report_counts_the_runs_that_typed_full_all_and_long() {
    let typed = |at, flags: &[&str]| {
        let mut entry = Entry::run("a", at, "books list");
        entry.flags = flags.iter().map(|flag| (*flag).to_owned()).collect();
        entry
    };
    let entries = [
        typed(1, &["--all", "--json"]),
        typed(2, &["--full", "--json", "--long"]),
        typed(3, &["--all"]),
    ];

    let report = report(&entries);

    let figures = figures(&report, "books list");
    assert_eq!((figures.full, figures.all, figures.long), (1, 2, 1));
}

#[test]
fn output_size_percentiles_leave_out_runs_that_did_not_reach_finish() {
    let mut early = Entry::run("a", 1, "books list");
    early.out_bytes = None;
    let mut help = Entry::run("a", 2, "books show");
    help.help = true;

    let report = report(&[early, help]);

    assert_eq!(figures(&report, "books list").p95_out_bytes, None);
    assert_eq!(figures(&report, "books show").p95_ms, None);
}

#[test]
fn the_95th_percentile_is_the_smallest_value_that_95_percent_are_no_larger_than() {
    let mut twenty: Vec<u64> = (1..=20).rev().collect();
    let mut twenty_one: Vec<u64> = (1..=21).collect();

    assert_eq!(p95(&mut []), None);
    assert_eq!(p95(&mut [7]), Some(7));
    assert_eq!(p95(&mut twenty), Some(19));
    assert_eq!(p95(&mut twenty_one), Some(20));
}
