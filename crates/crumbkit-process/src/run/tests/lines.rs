//! Tests of standard output delivered as lines, with real `sh` children.

use std::time::{Duration, Instant};

use super::super::{
    Capture, ChildFinished, DRAIN_GRACE, Limits, Outcome, OverLimit, RunError, Status, StopReason,
    run,
};
use crate::testing::sh;
use crate::{Cancel, Lines, line_queue};

/// How long a test waits for a line before it fails.
const WAIT: Duration = Duration::from_secs(5);

/// Returns the settings that deliver standard output as lines to a queue
/// of `most` lines within `line` and `total` bytes, and the caller's side.
fn lines(most: usize, line: usize, total: usize) -> (Limits, Lines) {
    let (queue, lines) = line_queue(most);
    let capture = Capture::Lines {
        queue,
        line,
        total,
        over: OverLimit::Stop,
    };
    (Limits::new().stdout(capture), lines)
}

/// Returns every line that `lines` still holds once its run has ended.
fn rest(lines: &Lines) -> Vec<String> {
    std::iter::from_fn(|| lines.recv_timeout(WAIT).unwrap())
        .map(|line| String::from_utf8(line).unwrap())
        .collect()
}

/// Runs `script` with `settings`.
fn run_sh(script: &str, settings: &Limits) -> ChildFinished {
    run(sh(script), settings).unwrap()
}

#[test]
fn each_line_reaches_the_caller_while_the_child_still_runs() {
    let (settings, lines) = lines(16, 1024, 1 << 20);
    let cancel = Cancel::new();
    let settings = settings.cancel(&cancel);
    let running = std::thread::spawn(move || run_sh("echo first; sleep 30", &settings));

    let first = lines.recv_timeout(WAIT);

    assert!(
        !running.is_finished(),
        "the run ended before the line arrived"
    );
    cancel.cancel();
    let finished = running.join().unwrap();
    assert_eq!(first, Ok(Some(b"first".to_vec())));
    assert_eq!(finished.outcome(), Outcome::Stopped(StopReason::Cancelled));
    assert_eq!(rest(&lines), Vec::<String>::new());
}

#[test]
fn a_last_line_with_no_newline_is_delivered_when_the_stream_ends() {
    let (settings, lines) = lines(16, 1024, 1 << 20);

    let finished = run_sh("printf 'one\\ntwo'", &settings);

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert_eq!(rest(&lines), ["one", "two"]);
    assert!(!finished.stdout().over_limit());
}

#[test]
fn a_line_longer_than_its_limit_stops_the_child_and_no_later_line_arrives() {
    let (settings, lines) = lines(16, 1024, 1 << 20);
    let started = Instant::now();

    let finished = run_sh(
        "echo short; head -c 5000 /dev/zero | tr '\\0' x; echo; echo after; sleep 30",
        &settings,
    );

    assert_eq!(finished.outcome(), Outcome::Stopped(StopReason::OverLimit));
    assert!(finished.stdout().over_limit());
    assert!(finished.stdout().long_line());
    assert_eq!(rest(&lines), ["short"]);
    assert!(started.elapsed() < WAIT);
}

#[test]
fn a_stream_longer_than_its_total_limit_stops_the_child_without_a_long_line() {
    let (settings, lines) = lines(16, 1024, 12);

    let finished = run_sh("printf 'aaaa\\nbbbb\\ncccc\\n'; sleep 30", &settings);

    assert_eq!(finished.outcome(), Outcome::Stopped(StopReason::OverLimit));
    assert!(finished.stdout().over_limit());
    assert!(!finished.stdout().long_line());
    assert_eq!(rest(&lines), ["aaaa", "bbbb"]);
}

#[test]
fn a_full_queue_after_a_stop_drops_lines_and_keeps_the_last_one_as_lost() {
    let (settings, lines) = lines(1, 1024, 1 << 20);
    let settings = settings.deadline(Instant::now() + Duration::from_millis(500));
    let started = Instant::now();

    let finished = run_sh("printf 'one\\ntwo\\nthree\\n'; sleep 30", &settings);

    assert_eq!(finished.outcome(), Outcome::TimedOut);
    assert!(started.elapsed() < WAIT);
    assert_eq!(rest(&lines), ["one"]);
    assert_eq!(lines.lost(), Some(b"three".to_vec()));
}

#[test]
fn a_caller_that_dropped_its_lines_lets_the_child_run_to_its_end() {
    let (settings, lines) = lines(1, 1024, 1 << 20);
    drop(lines);
    let settings = settings.stderr(Capture::Tail { limit: 64 });

    let finished = run_sh("seq 1 5000; echo done >&2; exit 4", &settings);

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(4)));
    assert_eq!(finished.stderr().bytes(), b"done\n");
}

#[test]
fn a_run_returns_when_its_child_exits_while_nobody_reads_a_full_queue() {
    let (settings, lines) = lines(1, 1024, 1 << 20);
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || done.send(run_sh("printf 'one\\ntwo\\nthree\\n'", &settings)));

    let finished = finished
        .recv_timeout(DRAIN_GRACE * 4)
        .expect("the run waited on its full queue after the child exited");

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert_eq!(rest(&lines), ["one"]);
    assert_eq!(lines.lost(), Some(b"three".to_vec()));
}

#[test]
fn the_last_line_a_full_queue_drops_after_the_child_exits_is_the_end_of_the_stream() {
    let (settings, lines) = lines(1, 1024, 1 << 30);
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || done.send(run_sh("seq 1 5000", &settings)));

    let finished = finished
        .recv_timeout(DRAIN_GRACE * 4)
        .expect("the run waited on its full queue after the child exited");

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert_eq!(rest(&lines), ["1"]);
    assert_eq!(lines.lost(), Some(b"5000".to_vec()));
}

#[test]
fn a_caller_who_keeps_reading_receives_every_line_of_a_child_that_exits_with_more_output_than_the_queue_holds()
 {
    let (settings, lines) = lines(4, 1024, 1 << 30);
    let running = std::thread::spawn(move || run_sh("seq 1 2000", &settings));

    let received = rest(&lines);

    let finished = running.join().unwrap();
    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert_eq!(received.len(), 2000);
    assert_eq!(received.first().map(String::as_str), Some("1"));
    assert_eq!(received.last().map(String::as_str), Some("2000"));
    assert_eq!(lines.lost(), None);
}

#[test]
fn the_lines_end_when_the_program_cannot_start() {
    let (settings, lines) = lines(16, 1024, 1 << 20);
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds a command whose program does not exist"
    )]
    let missing = std::process::Command::new("crumbkit-process-test-no-such-program");

    let refused = run(missing, &settings).unwrap_err();

    assert!(matches!(refused, RunError::Start(_)), "{refused:?}");
    assert_eq!(lines.recv_timeout(WAIT), Ok(None));
}

#[test]
fn the_lines_end_when_the_cancel_was_cancelled_before_the_start() {
    let (settings, lines) = lines(16, 1024, 1 << 20);
    let cancel = Cancel::new();
    cancel.cancel();

    let finished = run_sh("echo started", &settings.cancel(&cancel));

    assert_eq!(finished.outcome(), Outcome::Stopped(StopReason::Cancelled));
    assert_eq!(lines.recv_timeout(WAIT), Ok(None));
}
