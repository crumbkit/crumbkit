use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::{Cancel, Outcome, RunError, RunOptions, StopReason, after, kept, run, run_sh};
use crate::Status;
use crate::testing::{gone, pid_in};

/// Settings whose report sends what it was given down the returned channel.
fn reporting(at: Instant) -> (RunOptions, mpsc::Receiver<crate::Overdue>) {
    let (sender, receiver) = mpsc::channel();
    let options = kept().overdue(at, move |overdue| sender.send(overdue).unwrap());
    (options, receiver)
}

#[test]
fn a_run_past_its_overdue_time_reports_once_and_waits_for_the_child_to_exit() {
    let (options, reports) = reporting(after(0.2));
    let started = Instant::now();

    let finished = run_sh("sleep 1; echo done", &options);

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert!(finished.was_overdue());
    assert_eq!(finished.stdout().bytes(), b"done\n");
    assert!(started.elapsed() >= Duration::from_secs(1));
    let report = reports.try_recv().unwrap();
    assert!(report.elapsed() >= Duration::from_millis(150));
    assert!(reports.try_recv().is_err());
}

#[test]
fn an_overdue_run_never_signals_its_child() {
    let (options, _reports) = reporting(after(0.2));

    let finished = run_sh(
        "trap 'echo signalled' INT TERM HUP; sleep 1 & wait; exit 0",
        &options,
    );

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert!(finished.was_overdue());
    assert_eq!(finished.stdout().bytes(), b"");
}

#[test]
fn a_child_that_exits_before_its_overdue_time_is_never_reported() {
    let (options, reports) = reporting(after(30.0));

    let finished = run_sh("exit 4", &options);

    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(4)));
    assert!(!finished.was_overdue());
    assert!(reports.try_recv().is_err());
}

#[test]
fn the_report_gets_the_id_of_the_run_s_unreaped_child() {
    let (options, reports) = reporting(after(0.2));

    let finished = run_sh("echo $$; sleep 1", &options);

    let report = reports.try_recv().unwrap();
    let printed = pid_in(finished.stdout().bytes());
    assert_eq!(i64::from(report.id()), i64::from(printed));
    assert!(gone(printed));
}

#[test]
fn a_report_that_cancels_stops_the_run_with_stopped_cancelled() {
    let cancel = Cancel::new();
    let options = kept().cancel(&cancel).overdue(after(0.2), {
        let cancel = cancel.clone();
        move |_| cancel.cancel()
    });
    let started = Instant::now();

    let finished = run_sh("sleep 30", &options);

    assert_eq!(finished.outcome(), Outcome::Stopped(StopReason::Cancelled));
    assert!(finished.was_overdue());
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn an_overdue_time_after_the_deadline_never_reports_and_the_run_times_out() {
    let (options, reports) = reporting(after(5.0));

    let finished = run_sh("sleep 30", &options.deadline(after(0.2)));

    assert_eq!(finished.outcome(), Outcome::TimedOut);
    assert!(!finished.was_overdue());
    assert!(reports.try_recv().is_err());
}

#[test]
fn an_overdue_report_before_the_deadline_still_lets_the_deadline_stop_the_run() {
    let (options, reports) = reporting(after(0.2));

    let finished = run_sh("sleep 30", &options.deadline(after(0.6)));

    assert_eq!(finished.outcome(), Outcome::TimedOut);
    assert!(finished.was_overdue());
    assert!(reports.try_recv().is_ok());
}

#[test]
fn a_report_that_blocks_past_the_deadline_delays_the_stop_and_the_run_still_times_out() {
    let options = kept()
        .overdue(after(0.1), |_| {
            std::thread::sleep(Duration::from_millis(600))
        })
        .deadline(after(0.3));
    let started = Instant::now();

    let finished = run_sh("sleep 30", &options);

    assert_eq!(finished.outcome(), Outcome::TimedOut);
    assert!(finished.was_overdue());
    assert!(started.elapsed() >= Duration::from_millis(700));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn a_report_that_panics_stops_and_reaps_the_child_and_returns_an_error() {
    let path = std::env::temp_dir().join(format!("crumbkit-overdue-panic-{}", std::process::id()));
    let options = RunOptions::new().overdue(after(0.2), |_| panic!("the report failed"));
    let script = format!("echo $$ > {}; sleep 30", path.display());

    let refused = run(super::sh(&script), &options).unwrap_err();

    assert!(matches!(refused, RunError::Run(_)), "{refused:?}");
    let pid = pid_in(&std::fs::read(&path).unwrap());
    assert!(gone(pid));
    #[expect(
        clippy::disallowed_methods,
        reason = "the test removes the file that its child wrote"
    )]
    std::fs::remove_file(path).unwrap();
}
