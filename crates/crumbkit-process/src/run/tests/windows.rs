use std::time::{Duration, Instant};

use super::super::{Cancel, Capture, Exit, OverLimit, Run, Stop, run};
use crate::testing::{cmd, gone, pid_file, pid_in, remove, started_grandchild};
use crate::{Scope, Status};

/// Keeps up to 1 MiB of standard output whole.
const WHOLE: Capture = Capture::Whole {
    limit: 1 << 20,
    over: OverLimit::Stop,
};

/// Returns the standard output of a run as text.
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_child_gives_its_exit_code_and_its_standard_output() {
    let finished = run(cmd("echo out& exit 3"), &Run::new().stdout(WHOLE)).unwrap();

    assert_eq!(finished.exit(), Exit::Code(3));
    assert_eq!(finished.status(), Some(Status::Code(3)));
    assert_eq!(finished.stdout().bytes(), b"out\r\n");
}

#[test]
fn a_cancelled_child_whose_job_was_ended_gives_exit_code_1() {
    let cancel = Cancel::new();
    let canceller = std::thread::spawn({
        let cancel = cancel.clone();
        move || {
            std::thread::sleep(Duration::from_millis(500));
            cancel.cancel();
        }
    });

    let finished = run(
        cmd("ping -n 120 127.0.0.1 >NUL"),
        &Run::new().stdout(WHOLE).cancel(&cancel),
    )
    .unwrap();

    canceller.join().unwrap();
    assert_eq!(finished.exit(), Exit::Stopped(Stop::Cancelled));
    assert_eq!(finished.status(), Some(Status::Code(1)));
}

#[test]
fn ending_the_job_when_the_child_exits_ends_a_grandchild_that_holds_standard_output() {
    // The grandchild sleeps 120 s while it holds standard output, so a run
    // that waited for it would take that long. PowerShell can take several
    // seconds to start on a cold machine, so the bound leaves room for that.
    let file = pid_file("job-ends");
    let started = Instant::now();

    let finished = run(
        cmd(&started_grandchild(&file, 120)),
        &Run::new().stdout(WHOLE),
    )
    .unwrap();

    let took = started.elapsed();
    remove(&file);
    let out = text(finished.stdout().bytes());
    assert_eq!(finished.exit(), Exit::Code(0));
    assert!(out.contains("started"), "{out:?}");
    assert!(!out.contains("woke"), "{out:?}");
    let grandchild = pid_in(finished.stdout().bytes());
    assert!(took < Duration::from_secs(60), "the run took {took:?}");
    assert!(
        gone(grandchild),
        "the grandchild {grandchild} outlived the run"
    );
}

#[test]
fn in_the_child_scope_the_run_waits_for_a_grandchild_that_holds_standard_output() {
    // With no job, the grandchild outlives the child, and the reader of
    // standard output waits until the grandchild closes it by exiting.
    let file = pid_file("child-scope");
    let started = Instant::now();

    let finished = run(
        cmd(&started_grandchild(&file, 3)),
        &Run::new().stdout(WHOLE).scope(Scope::Child),
    )
    .unwrap();

    let took = started.elapsed();
    remove(&file);
    let out = text(finished.stdout().bytes());
    assert_eq!(finished.exit(), Exit::Code(0));
    assert!(out.contains("started"), "{out:?}");
    assert!(
        out.contains("woke"),
        "the grandchild did not outlive the child: {out:?}"
    );
    assert!(took >= Duration::from_secs(3), "the run took {took:?}");
    let grandchild = pid_in(finished.stdout().bytes());
    assert!(
        gone(grandchild),
        "the grandchild {grandchild} is still running"
    );
}

#[test]
fn a_deadline_ends_the_job_with_the_grandchild_in_it() {
    let file = pid_file("deadline");
    let script = format!(
        "{} & ping -n 120 127.0.0.1 >NUL",
        started_grandchild(&file, 120)
    );
    let started = Instant::now();

    let finished = run(
        cmd(&script),
        &Run::new()
            .stdout(WHOLE)
            .deadline(Instant::now() + Duration::from_secs(20)),
    )
    .unwrap();

    let took = started.elapsed();
    remove(&file);
    let out = text(finished.stdout().bytes());
    assert_eq!(finished.exit(), Exit::TimedOut);
    assert!(out.contains("started"), "{out:?}");
    let grandchild = pid_in(finished.stdout().bytes());
    assert!(took < Duration::from_secs(60), "the run took {took:?}");
    assert!(
        gone(grandchild),
        "the grandchild {grandchild} outlived the run"
    );
}
