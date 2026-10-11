use std::collections::{HashMap, VecDeque};
use std::io::{BufReader, Cursor};
use std::time::{Duration, UNIX_EPOCH};

use proptest::prelude::*;
use serde_json::json;

use super::entry::Entry;
use super::{LONGEST, Report, Tally, read};

/// Returns one line of the log for a run of `command` that ended `at`
/// seconds after the Unix epoch.
fn line(at: u64, command: &str) -> String {
    let at = humantime::format_rfc3339_millis(UNIX_EPOCH + Duration::from_secs(at));
    let record = json!({
        "schema": "crumbkit/usage/v1",
        "tool": "shelf",
        "at": at.to_string(),
        "ms": 10,
        "caller": "person",
        "session": "0123456789abcdef",
        "args": "fedcba9876543210",
        "trace": "4bf92f3577b34da6a3ce929d0e0e4736",
        "run": "00f067aa0ba902b7",
        "command": command,
        "exit": 0,
    });
    format!("{record}\n")
}

/// Returns the report of `log` for the runs that ended at or after `since`
/// seconds after the Unix epoch.
fn report(log: &[u8], since: u64) -> Report {
    let mut tally = Tally::default();
    read(Cursor::new(log), since * 1000, &mut tally);
    tally.report()
}

/// Returns the command paths in `report`.
fn paths(report: &Report) -> Vec<&str> {
    report
        .commands
        .iter()
        .map(|figures| figures.command.as_str())
        .collect()
}

#[test]
fn the_reader_adds_every_record_of_the_log() {
    let log = [line(1, "books list"), line(2, "books show")].concat();

    let report = report(log.as_bytes(), 0);

    assert_eq!(report.runs, 2);
    assert_eq!(paths(&report), ["books list", "books show"]);
}

#[test]
fn the_reader_leaves_out_runs_that_ended_before_the_start_of_the_report() {
    let log = [line(99, "books list"), line(100, "books show")].concat();

    let report = report(log.as_bytes(), 100);

    assert_eq!(paths(&report), ["books show"]);
}

#[test]
fn the_reader_skips_lines_that_are_not_records_and_reads_the_lines_after_them() {
    let torn = line(3, "books show");
    let log = [
        line(1, "books list"),
        "not json\n".to_owned(),
        "\n".to_owned(),
        line(2, "books list"),
        torn[..torn.len() / 2].to_owned(),
    ]
    .concat();

    let report = report(log.as_bytes(), 0);

    assert_eq!(report.runs, 2);
}

#[test]
fn a_line_longer_than_any_record_is_skipped_and_the_next_line_is_read() {
    let damaged = format!("{}\n", "x".repeat(3 * LONGEST));
    let log = [damaged, line(1, "books list")].concat();

    let report = report(log.as_bytes(), 0);

    assert_eq!(paths(&report), ["books list"]);
}

#[test]
fn a_line_of_the_longest_length_that_the_log_writes_is_read() {
    let short = line(1, "books list");
    let padding = " ".repeat(LONGEST - short.len());
    let longest = format!("{}{padding}\n", short.trim_end());
    assert_eq!(longest.len(), LONGEST);

    let report = report(longest.as_bytes(), 0);

    assert_eq!(report.runs, 1);
}

#[test]
fn the_reader_reads_through_a_small_buffer_as_through_a_large_one() {
    let log = [line(1, "books list"), line(2, "books show")].concat();
    let mut tally = Tally::default();

    read(BufReader::with_capacity(7, log.as_bytes()), 0, &mut tally);

    assert_eq!(tally.report(), report(log.as_bytes(), 0));
}

/// Returns `entry` as the line of the log that records it.
fn line_of(entry: &Entry) -> String {
    let at = humantime::format_rfc3339_millis(UNIX_EPOCH + Duration::from_millis(entry.at));
    let mut record = json!({
        "schema": "crumbkit/usage/v1",
        "tool": "shelf",
        "at": at.to_string(),
        "ms": entry.ms,
        "caller": entry.caller,
        "session": entry.session,
        "args": entry.args,
        "trace": "4bf92f3577b34da6a3ce929d0e0e4736",
        "run": "00f067aa0ba902b7",
        "flags": entry.flags,
        "truncated": entry.truncated,
        "interrupted": entry.interrupted,
    });
    let optional = [
        (
            "command",
            entry.command.as_ref().map(|command| json!(command)),
        ),
        ("help", entry.help.then(|| json!("short"))),
        ("ids", entry.ids.map(|ids| json!(ids))),
        ("nearest", entry.nearest.as_ref().map(|flag| json!(flag))),
        ("exit", entry.exit.map(|exit| json!(exit))),
        ("code", entry.code.as_ref().map(|code| json!(code))),
        ("out_bytes", entry.out_bytes.map(|bytes| json!(bytes))),
    ];
    for (field, value) in optional {
        if let Some(value) = value {
            record[field] = value;
        }
    }
    format!("{record}\n")
}

/// The commands that generated runs choose from, with the tool's root.
const COMMANDS: [&str; 3] = ["books list", "books show", ""];

/// Generates the runs of a log: up to 60 runs from three sessions, each
/// session's runs in the order of their end times, which are 0 to 150
/// seconds apart within the session. A run prints help, is refused, fails,
/// shows a cut-short listing or succeeds, with up to two ids and some of
/// `--all`, `--limit` and `--full`, so every figure and finding occurs.
fn runs() -> impl Strategy<Value = Vec<Entry>> {
    let run = (
        0..3_usize,
        0..150_u64,
        0..3_usize,
        0..5_u8,
        0..3_u64,
        0..4_usize,
    );
    prop::collection::vec(run, 0..60).prop_map(|runs| {
        let mut clocks = [1_000_u64; 3];
        runs.into_iter()
            .map(|(session, gap, command, outcome, ids, flags)| {
                clocks[session] += gap;
                let mut entry =
                    Entry::run(&format!("s{session}"), clocks[session], COMMANDS[command]);
                entry.args = format!("{command} {outcome} {ids} {flags}");
                entry.ids = (command == 1).then_some(ids);
                entry.flags = [&[][..], &["--all"], &["--limit"], &["--full"]][flags]
                    .iter()
                    .map(|flag: &&str| (*flag).to_owned())
                    .collect();
                match outcome {
                    0 => entry.help = true,
                    1 => {
                        entry.exit = Some(2);
                        entry.nearest = Some("--all".to_owned());
                    }
                    2 => {
                        entry.exit = Some(1);
                        entry.code = Some("shelf::books::not_found".to_owned());
                    }
                    3 => entry.truncated = true,
                    _ => entry.out_bytes = Some(ids * 30_000),
                }
                entry
            })
            .collect()
    })
}

/// Generates the runs of [`runs`] together with a second order of the same
/// runs, which interleaves the sessions differently and keeps each
/// session's runs in their order.
fn reordered() -> impl Strategy<Value = (Vec<Entry>, Vec<Entry>)> {
    runs().prop_flat_map(|runs| {
        let sessions: Vec<String> = runs.iter().map(|entry| entry.session.clone()).collect();
        (Just(runs), Just(sessions).prop_shuffle()).prop_map(|(runs, sessions)| {
            // Each session's runs, in order, wait in a queue, and the shuffled
            // session names take the next run from their session's queue.
            let mut queues: HashMap<&str, VecDeque<&Entry>> = HashMap::new();
            for entry in &runs {
                queues.entry(&entry.session).or_default().push_back(entry);
            }
            let reordered = sessions
                .iter()
                .filter_map(|name| queues.get_mut(name.as_str())?.pop_front().cloned())
                .collect();
            (runs, reordered)
        })
    })
}

/// Returns the log that records `runs` in order.
fn log_of(runs: &[Entry]) -> String {
    runs.iter().map(line_of).collect()
}

proptest! {
    /// The runs of different sessions interleave in the log in whatever
    /// order they ended, so the report must not depend on that order. Each
    /// session's own runs stay in the order of their end times, because the
    /// findings read a session's runs in that order.
    #[test]
    fn the_report_is_the_same_for_any_interleaving_of_the_sessions(
        (runs, reordered) in reordered(),
    ) {
        prop_assert_eq!(
            report(log_of(&runs).as_bytes(), 0),
            report(log_of(&reordered).as_bytes(), 0),
        );
    }

    /// A tally keeps everything that the report needs between reads, so
    /// reading a log in two parts into one tally reports what reading it
    /// whole does.
    #[test]
    fn reading_a_log_in_two_parts_into_one_tally_reports_what_reading_it_whole_does(
        (runs, split) in runs().prop_flat_map(|runs| {
            let len = runs.len();
            (Just(runs), 0..=len)
        }),
    ) {
        let mut tally = Tally::default();
        read(Cursor::new(log_of(&runs[..split])), 0, &mut tally);
        read(Cursor::new(log_of(&runs[split..])), 0, &mut tally);

        prop_assert_eq!(tally.report(), report(log_of(&runs).as_bytes(), 0));
    }

    /// The reader reads back every run that a line of the log records.
    #[test]
    fn every_generated_line_reads_back_as_its_run(runs in runs()) {
        let report = report(log_of(&runs).as_bytes(), 0);

        prop_assert_eq!(report.runs, runs.len() as u64);
    }
}
