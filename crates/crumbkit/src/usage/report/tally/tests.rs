use super::{HEAVY_OUTPUT, SLOW, Tally, p95};
use crate::usage::report::entry::Entry;
use crate::usage::report::{Count, Figures, Finding, Kind, Report};

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
fn the_help_pages_a_session_read_since_its_last_other_run_count_towards_the_next_command() {
    let help = |session, at, command| {
        let mut entry = Entry::run(session, at, command);
        entry.help = true;
        entry
    };
    let entries = [
        help("a", 1, ""),
        help("b", 2, ""),
        help("a", 3, "books"),
        help("a", 4, "books show"),
        Entry::run("a", 5, "books show"),
        Entry::run("a", 6, "books show"),
        help("a", 7, "books list"),
        Entry::run("b", 8, "books show"),
        Entry::run("a", 9, "books list"),
    ];

    let report = report(&entries);

    assert_eq!(figures(&report, "books show").help_before, 4);
    assert_eq!(figures(&report, "books list").help_before, 1);
    assert_eq!(figures(&report, "").help_before, 0);
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
fn the_report_counts_the_runs_of_each_exit_code_from_3_in_code_order() {
    let exited = |at, exit| {
        let mut entry = Entry::run("a", at, "books import");
        entry.exit = Some(exit);
        entry
    };
    let mut help = exited(6, 124);
    help.help = true;

    let report = report(&[
        exited(1, 124),
        exited(2, 3),
        exited(3, 124),
        exited(4, 1),
        exited(5, 2),
        help,
    ]);

    assert_eq!(figures(&report, "books import").exits, [(3, 1), (124, 2)]);
}

#[test]
fn the_report_counts_the_runs_that_tolerated_each_code_the_most_first() {
    let tolerated = |at, codes: &[&str]| {
        let mut entry = Entry::run("a", at, "books list");
        entry.tolerated = codes.iter().map(|code| (*code).to_owned()).collect();
        entry
    };

    let report = report(&[
        tolerated(1, &["shelf::network::timed_out"]),
        tolerated(
            2,
            &["shelf::catalogue::locked", "shelf::network::timed_out"],
        ),
        tolerated(3, &["shelf::catalogue::locked", "shelf::catalogue::locked"]),
        tolerated(4, &["shelf::network::timed_out"]),
    ]);

    assert_eq!(
        report.tolerated,
        [
            count("shelf::network::timed_out", 3),
            count("shelf::catalogue::locked", 2)
        ]
    );
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

/// Returns the kind, command and runs of each finding of `report`, in order.
fn kinds(report: &Report) -> Vec<(Kind, &str, u64)> {
    report
        .findings
        .iter()
        .map(|finding| (finding.kind, finding.command.as_str(), finding.runs))
        .collect()
}

#[test]
fn a_command_with_a_refused_run_is_a_usage_error_finding_that_counts_them() {
    let mut refused = Entry::run("a", 1, "books show");
    refused.exit = Some(2);

    let report = report(&[refused, Entry::run("a", 2, "books show")]);

    assert_eq!(kinds(&report), [(Kind::UsageError, "books show", 1)]);
}

#[test]
fn a_command_whose_95th_percentile_output_reaches_the_limit_is_heavy_and_counts_the_heavy_runs() {
    let sized = |at, bytes| {
        let mut entry = Entry::run("a", at, "books list");
        entry.out_bytes = Some(bytes);
        entry
    };
    let heavy: Vec<Entry> = (0..20).map(|at| sized(at, HEAVY_OUTPUT)).collect();
    let under: Vec<Entry> = (0..20).map(|at| sized(at, HEAVY_OUTPUT - 1)).collect();
    let one_heavy_in_twenty: Vec<Entry> = (0..20)
        .map(|at| sized(at, if at == 0 { HEAVY_OUTPUT } else { 10 }))
        .collect();

    assert_eq!(
        kinds(&report(&heavy)),
        [(Kind::HeavyOutput, "books list", 20)]
    );
    assert_eq!(kinds(&report(&under)), []);
    assert_eq!(kinds(&report(&one_heavy_in_twenty)), []);
}

#[test]
fn a_command_whose_95th_percentile_wall_time_reaches_a_second_is_slow_and_counts_the_slow_runs() {
    let timed = |at, ms| {
        let mut entry = Entry::run("a", at, "books list");
        entry.ms = ms;
        entry
    };

    let slow = report(&[timed(1, SLOW), timed(2, 10)]);
    let quick = report(&[timed(1, SLOW - 1)]);

    assert_eq!(kinds(&slow), [(Kind::SlowCommand, "books list", 1)]);
    assert_eq!(kinds(&quick), []);
}

#[test]
fn findings_come_most_runs_first_then_in_the_order_of_their_kind_and_command() {
    let refused = |at, command| {
        let mut entry = Entry::run("a", at, command);
        entry.exit = Some(2);
        entry
    };
    let mut slow = Entry::run("a", 10, "books list");
    slow.ms = SLOW;
    let entries = [
        refused(1, "books show"),
        refused(2, "books list"),
        refused(3, "books list"),
        slow,
    ];

    let report = report(&entries);

    assert_eq!(
        kinds(&report),
        [
            (Kind::UsageError, "books list", 2),
            (Kind::UsageError, "books show", 1),
            (Kind::SlowCommand, "books list", 1),
        ]
    );
}

#[test]
fn a_session_finding_reaches_the_report_with_the_findings_of_its_figures() {
    let mut help_pages: Vec<Entry> = (0..3)
        .map(|at| {
            let mut entry = Entry::run("a", at, "books list");
            entry.help = true;
            entry
        })
        .collect();
    help_pages.push(Entry::run("b", 4, "books list"));

    let report = report(&help_pages);

    assert_eq!(
        report.findings,
        [Finding {
            kind: Kind::HelpLoop,
            command: "books list".to_owned(),
            runs: 3,
            code: None,
        }]
    );
}
