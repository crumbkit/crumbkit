//! Tests that a run leaves no process of the child's group when it returns.
//!
//! The killed processes are orphans, so these tests need an init process
//! that reaps orphans. In a container whose first process reaps nothing,
//! each run waits the whole `REAP_GRACE` and the tests fail.

use std::time::Duration;

use rustix::io::Errno;
use rustix::process::{Pid, test_kill_process_group};

use super::{after, kept, run_sh};
use crate::testing::pid_in;
use crate::{Escalation, Outcome, Signal};

/// How many runs each test makes, so a gap that only some runs hit shows.
const RUNS: usize = 20;

/// Asserts that the kernel finds no process in the group `group` at once,
/// not even one that has exited and waits for its parent to reap it.
fn assert_empty(group: i32) {
    let group = Pid::from_raw(group).unwrap();
    assert_eq!(
        test_kill_process_group(group),
        Err(Errno::SRCH),
        "a process of the group is left after the run returned"
    );
}

#[test]
fn a_run_leaves_no_process_of_the_group_after_its_child_ends() {
    for _ in 0..RUNS {
        let finished = run_sh("echo $$; sleep 30 & sleep 30 & exit 0", &kept());

        assert_empty(pid_in(finished.stdout().bytes()));
    }
}

#[test]
fn a_run_leaves_no_process_of_the_group_when_a_descendant_forks_in_a_loop() {
    for _ in 0..RUNS {
        let finished = run_sh("echo $$; (while :; do (:) & done) & sleep 0.05", &kept());

        assert_empty(pid_in(finished.stdout().bytes()));
    }
}

#[test]
fn a_run_leaves_no_process_of_the_group_after_it_stops_its_child() {
    let escalation = Escalation::default().step(Signal::Terminate, Duration::from_millis(100));
    for _ in 0..RUNS {
        let options = kept().deadline(after(0.1)).escalation(escalation.clone());

        let finished = run_sh("echo $$; sleep 30 & sleep 30 & sleep 30", &options);

        assert_eq!(finished.outcome(), Outcome::TimedOut);
        assert_empty(pid_in(finished.stdout().bytes()));
    }
}
