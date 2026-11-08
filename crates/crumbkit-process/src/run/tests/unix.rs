use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::super::{
    Cancel, Capture, ChildFinished, DRAIN_GRACE, Escalation, Input, Limits, Outcome, OverLimit,
    RunError, Stop, control, run,
};
use crate::Status;
use crate::notice::Notice;
use crate::testing::{gone, kill, pid_in, sh};
use crate::{Group, Scope, Signal};

/// Keeps up to 1 MiB of each stream whole and stops the child past it.
const WHOLE: Capture = Capture::Whole {
    limit: 1 << 20,
    over: OverLimit::Stop,
};

/// The settings that keep both streams whole.
fn kept() -> Limits {
    Limits::new().stdout(WHOLE).stderr(WHOLE)
}

/// Runs `script` with `settings`.
fn run_sh(script: &str, settings: &Limits) -> ChildFinished {
    run(sh(script), settings).unwrap()
}

/// Returns `seconds` from now.
fn after(seconds: f64) -> Instant {
    Instant::now() + Duration::from_secs_f64(seconds)
}

#[test]
fn a_child_gives_its_exit_code_and_both_streams_whole() {
    let finished = run_sh("echo out; echo err >&2; exit 3", &kept());

    assert_eq!(finished.exit(), Outcome::Code(3));
    assert_eq!(finished.status(), Some(Status::Code(3)));
    assert_eq!(finished.stdout().bytes(), b"out\n");
    assert_eq!(finished.stderr().bytes(), b"err\n");
    assert!(!finished.stdout().over_limit());
}

#[test]
fn a_child_that_a_signal_ended_gives_the_signals_number() {
    let finished = run_sh("kill -TERM $$", &kept());

    assert_eq!(finished.exit(), Outcome::Signal(15));
}

#[test]
fn a_discarded_stream_keeps_nothing_and_has_no_first_byte() {
    let finished = run_sh("echo out; echo err >&2", &Limits::new());

    assert_eq!(finished.exit(), Outcome::Code(0));
    assert_eq!(finished.stdout().bytes(), b"");
    assert_eq!(finished.stderr().bytes(), b"");
    assert_eq!(finished.first_stdout(), None);
}

#[test]
fn a_program_that_cannot_start_is_a_start_error() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test builds a command whose program does not exist"
    )]
    let missing = std::process::Command::new("crumbkit-process-test-no-such-program");

    let refused = run(missing, &kept()).unwrap_err();

    assert!(matches!(refused, RunError::Start(_)), "{refused:?}");
}

#[test]
fn stdin_bytes_reach_the_child_and_end_with_the_end_of_its_input() {
    let finished = run_sh("cat", &kept().stdin(Input::Bytes(b"given\n".to_vec())));

    assert_eq!(finished.exit(), Outcome::Code(0));
    assert_eq!(finished.stdout().bytes(), b"given\n");
}

#[test]
fn a_child_that_never_reads_its_large_input_still_ends_the_run() {
    let input = Input::Bytes(vec![b'x'; 4 << 20]);
    let started = Instant::now();

    let finished = run_sh("exit 0", &kept().stdin(input));

    assert_eq!(finished.exit(), Outcome::Code(0));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_child_past_its_deadline_is_stopped_with_the_process_it_started() {
    let started = Instant::now();

    let finished = run_sh("sleep 30 & echo $!; wait", &kept().deadline(after(0.3)));

    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(5));
    let sleeper = pid_in(finished.stdout().bytes());
    assert!(gone(sleeper), "the sleep {sleeper} outlived the run");
}

#[test]
fn a_deadline_that_passes_after_the_child_exited_gives_the_childs_exit() {
    let mut command = sh("exit 3");
    command.stdout(Stdio::null());
    let mut group = Group::start(command, Scope::Group).unwrap();
    group.wait_exit().unwrap();
    // No waiter has said that the child exited, as when the deadline wins
    // the race with the waiter. The waiter of this test says so later.
    let notice = Arc::new(Notice::default());
    let waiter = std::thread::spawn({
        let notice = Arc::clone(&notice);
        move || {
            std::thread::sleep(Duration::from_millis(100));
            notice.set(|flags| flags.exited = true);
        }
    });

    let stopped = control(&group, &notice, &Limits::new().deadline(Instant::now()));

    waiter.join().unwrap();
    assert_eq!(stopped, None);
    assert_eq!(group.reap().unwrap().code(), Some(3));
}

#[test]
fn a_background_process_that_holds_standard_output_is_killed_when_the_child_exits() {
    let started = Instant::now();

    let finished = run_sh("sleep 30 & echo $!", &kept());

    assert_eq!(finished.exit(), Outcome::Code(0));
    assert!(started.elapsed() < Duration::from_secs(5));
    let sleeper = pid_in(finished.stdout().bytes());
    assert!(gone(sleeper), "the sleep {sleeper} outlived the run");
}

#[test]
fn a_process_that_left_the_group_holding_a_pipe_ends_the_run_after_the_drain_grace() {
    // perl leaves the group with setsid, prints its id and keeps the
    // inherited standard output open while it sleeps. The shell waits before
    // it exits, so perl has left the group before the run kills the group.
    let script = "perl -MPOSIX -e 'POSIX::setsid(); $| = 1; print qq($$\\n); sleep 30' & \
                  sleep 0.5; echo started";
    let started = Instant::now();

    let finished = run_sh(script, &kept().deadline(after(20.0)));

    let took = started.elapsed();
    let text = String::from_utf8_lossy(finished.stdout().bytes()).into_owned();
    // The process that left the group is not the run's to stop, so the test
    // stops it.
    let left = pid_in(finished.stdout().bytes());
    kill(left);
    assert_eq!(finished.exit(), Outcome::Code(0));
    assert!(text.contains("started"), "{text:?}");
    assert!(took >= DRAIN_GRACE, "the run drained for {took:?}");
    assert!(took < DRAIN_GRACE * 4, "the run drained for {took:?}");
}

#[test]
fn a_child_that_ignores_interrupt_and_terminate_is_killed_after_both_graces() {
    let grace = Duration::from_millis(300);
    let escalation = Escalation::default()
        .step(Signal::Interrupt, grace)
        .step(Signal::Terminate, grace);
    let started = Instant::now();

    let finished = run_sh(
        "trap '' INT TERM; sleep 30",
        &kept().deadline(after(0.1)).escalation(escalation),
    );

    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert!(started.elapsed() >= grace * 2, "{:?}", started.elapsed());
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_child_that_handles_terminate_exits_at_that_step_and_is_not_killed() {
    let escalation = Escalation::default().step(Signal::Terminate, Duration::from_secs(10));
    let started = Instant::now();

    let finished = run_sh(
        "trap 'sleep 0.3; echo bye >&2; exit 7' TERM; sleep 30 & wait",
        &kept().deadline(after(0.2)).escalation(escalation),
    );

    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert_eq!(finished.stderr().bytes(), b"bye\n");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_child_stopped_by_job_control_still_handles_the_graceful_signal_at_its_step() {
    let escalation = Escalation::default().step(Signal::Terminate, Duration::from_secs(10));
    let started = Instant::now();

    let finished = run_sh(
        "trap 'echo bye >&2; exit 7' TERM; kill -STOP $$; sleep 30",
        &kept().deadline(after(0.3)).escalation(escalation),
    );

    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert_eq!(finished.stderr().bytes(), b"bye\n");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn in_the_child_scope_a_child_stopped_by_job_control_still_handles_the_graceful_signal() {
    let escalation = Escalation::default().step(Signal::Terminate, Duration::from_secs(10));
    let started = Instant::now();

    let finished = run_sh(
        "trap 'echo bye >&2; exit 7' TERM; kill -STOP $$; sleep 30",
        &kept()
            .deadline(after(0.3))
            .escalation(escalation)
            .scope(Scope::Child),
    );

    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert_eq!(finished.stderr().bytes(), b"bye\n");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_grace_too_long_for_the_clock_waits_for_the_child_without_a_panic() {
    let escalation = Escalation::default().step(Signal::Terminate, Duration::MAX);
    let started = Instant::now();

    let finished = run_sh(
        "trap 'exit 7' TERM; sleep 30 & wait",
        &kept().deadline(after(0.2)).escalation(escalation),
    );

    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn standard_output_past_its_limit_stops_the_child() {
    let limit = Capture::Whole {
        limit: 1024,
        over: OverLimit::Stop,
    };
    let started = Instant::now();

    let finished = run_sh("head -c 4096 /dev/zero; sleep 30", &kept().stdout(limit));

    assert_eq!(finished.exit(), Outcome::Stopped(Stop::OverLimit));
    assert_eq!(finished.status(), Some(Status::Signal(9)));
    assert!(finished.stdout().over_limit());
    assert_eq!(finished.stdout().bytes().len(), 1024);
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn standard_output_past_its_limit_into_a_sink_is_read_to_its_end() {
    let limit = Capture::Whole {
        limit: 1024,
        over: OverLimit::Sink,
    };

    let finished = run_sh(
        "head -c 300000 /dev/zero; echo done >&2",
        &kept().stdout(limit),
    );

    assert_eq!(finished.exit(), Outcome::Code(0));
    assert!(finished.stdout().over_limit());
    assert_eq!(finished.stdout().bytes().len(), 1024);
    assert_eq!(finished.stderr().bytes(), b"done\n");
}

#[test]
fn standard_error_keeps_its_last_bytes() {
    let finished = run_sh(
        "head -c 100000 /dev/zero >&2; printf 'the end' >&2",
        &kept().stderr(Capture::Tail { limit: 7 }),
    );

    assert_eq!(finished.stderr().bytes(), b"the end");
    assert!(finished.stderr().over_limit());
}

#[test]
fn the_time_of_the_first_byte_of_standard_output_is_measured_from_the_start() {
    let finished = run_sh("sleep 0.3; echo late", &kept());

    let first = finished.first_stdout().unwrap();
    assert!(first >= Duration::from_millis(300), "{first:?}");
    assert!(first < Duration::from_secs(5), "{first:?}");
    assert_eq!(run_sh("exit 0", &kept()).first_stdout(), None);
}

#[test]
fn a_cancel_from_another_thread_stops_the_child() {
    let (cancel, canceller) = cancel_soon();
    let started = Instant::now();

    let finished = run_sh("sleep 30", &kept().cancel(&cancel));

    canceller.join().unwrap();
    assert_eq!(finished.exit(), Outcome::Stopped(Stop::Cancelled));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_run_with_a_cancel_that_is_already_cancelled_starts_no_child() {
    let cancel = Cancel::new();
    cancel.cancel();

    let finished = run_sh("echo started", &kept().cancel(&cancel));

    assert_eq!(finished.exit(), Outcome::Stopped(Stop::Cancelled));
    assert_eq!(finished.status(), None);
    assert_eq!(finished.stdout().bytes(), b"");
}

/// Returns a cancel that is cancelled 200 ms from now, and the thread that
/// cancels it.
///
/// The tests that stop a child with SIGINT need the test process to have
/// SIGINT at its default action, because a child inherits an ignored
/// SIGINT and then neither ends on it nor traps it.
fn cancel_soon() -> (Cancel, std::thread::JoinHandle<()>) {
    let cancel = Cancel::new();
    let canceller = std::thread::spawn({
        let cancel = cancel.clone();
        move || {
            std::thread::sleep(Duration::from_millis(200));
            cancel.cancel();
        }
    });
    (cancel, canceller)
}

#[test]
fn a_cancelled_child_that_sigint_ended_gives_that_signal_and_the_shell_code_130() {
    let (cancel, canceller) = cancel_soon();
    let escalation = Escalation::default().step(Signal::Interrupt, Duration::from_secs(10));

    let finished = run_sh(
        "exec sleep 30",
        &kept().cancel(&cancel).escalation(escalation),
    );

    canceller.join().unwrap();
    assert_eq!(finished.exit(), Outcome::Stopped(Stop::Cancelled));
    assert_eq!(finished.status(), Some(Status::Signal(2)));
    assert_eq!(finished.status().map(Status::shell_code), Some(130));
}

#[test]
fn a_cancelled_child_that_handles_sigint_gives_its_own_exit_code() {
    let (cancel, canceller) = cancel_soon();
    let escalation = Escalation::default().step(Signal::Interrupt, Duration::from_secs(10));

    let finished = run_sh(
        "trap 'exit 7' INT; sleep 30 & wait",
        &kept().cancel(&cancel).escalation(escalation),
    );

    canceller.join().unwrap();
    assert_eq!(finished.exit(), Outcome::Stopped(Stop::Cancelled));
    assert_eq!(finished.status(), Some(Status::Code(7)));
}

#[test]
fn a_timed_out_child_gives_the_status_that_its_stop_caused() {
    let escalation = Escalation::default().step(Signal::Terminate, Duration::from_secs(10));

    let finished = run_sh(
        "exec sleep 30",
        &kept().deadline(after(0.2)).escalation(escalation),
    );

    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert_eq!(finished.status(), Some(Status::Signal(15)));
}

#[test]
fn in_the_child_scope_the_deadline_stops_only_the_child_and_the_drain_grace_ends_the_run() {
    let started = Instant::now();

    let finished = run_sh(
        "sleep 30 & echo $!; wait",
        &kept().deadline(after(0.2)).scope(Scope::Child),
    );

    let took = started.elapsed();
    let sleeper = pid_in(finished.stdout().bytes());
    let survived =
        rustix::process::test_kill_process(rustix::process::Pid::from_raw(sleeper).unwrap())
            .is_ok();
    kill(sleeper);
    assert_eq!(finished.exit(), Outcome::TimedOut);
    assert!(survived, "the run stopped the sleep that the child started");
    assert!(took < DRAIN_GRACE * 4, "the run took {took:?}");
}
