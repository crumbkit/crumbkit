//! Tests of the report of processes that outlive a run's child.

use std::time::Duration;

use super::{after, kept, run_sh};
use crate::testing::{gone, pid_in};
use crate::{Escalation, Outcome, Signal};

#[test]
fn a_run_that_asks_says_when_a_process_outlived_the_child() {
    let finished = run_sh("sleep 30 & echo $!", &kept().report_survivors());

    assert!(finished.survivors());
    assert!(gone(pid_in(finished.stdout().bytes())));
}

#[test]
fn a_run_that_asks_says_nothing_outlived_a_child_that_ended_alone() {
    let finished = run_sh("exit 0", &kept().report_survivors());

    assert!(!finished.survivors());
}

#[test]
fn a_run_that_does_not_ask_never_reports_survivors() {
    let finished = run_sh("sleep 30 & echo $!", &kept());

    assert!(!finished.survivors());
    assert!(gone(pid_in(finished.stdout().bytes())));
}

#[test]
fn a_run_that_stopped_its_child_never_reports_survivors() {
    let escalation = Escalation::default().step(Signal::Terminate, Duration::from_millis(200));
    let options = kept()
        .report_survivors()
        .deadline(after(0.3))
        .escalation(escalation);

    // The descendant ignores the signal, so it is alive when the child ends.
    let finished = run_sh(
        "(trap '' TERM; exec sleep 30) & echo $!; sleep 30",
        &options,
    );

    assert_eq!(finished.outcome(), Outcome::TimedOut);
    assert!(!finished.survivors());
    assert!(gone(pid_in(finished.stdout().bytes())));
}
