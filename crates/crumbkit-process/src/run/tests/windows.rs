use std::process::Command;
use std::time::{Duration, Instant};

use super::super::{Capture, Exit, OverLimit, Run, run};

/// Keeps up to 1 MiB of standard output whole.
const WHOLE: Capture = Capture::Whole {
    limit: 1 << 20,
    over: OverLimit::Stop,
};

/// Returns a command that runs `script` with `cmd`.
fn cmd(script: &str) -> Command {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests build the commands that the crate runs"
    )]
    let mut cmd = Command::new("cmd");
    cmd.args(["/C", script]);
    cmd
}

#[test]
fn a_child_gives_its_exit_code_and_its_standard_output() {
    let finished = run(cmd("echo out& exit 3"), &Run::new().stdout(WHOLE)).unwrap();

    assert_eq!(finished.exit(), Exit::Code(3));
    assert_eq!(finished.stdout().bytes(), b"out\r\n");
}

#[test]
fn ending_the_job_when_the_child_exits_ends_a_grandchild_that_holds_standard_output() {
    // `start /B` leaves ping running in the job with the child's standard
    // output, so the read ends only when the job ends.
    let started = Instant::now();

    let finished = run(
        cmd("start /B ping -n 30 127.0.0.1& echo started"),
        &Run::new().stdout(WHOLE),
    )
    .unwrap();

    assert_eq!(finished.exit(), Exit::Code(0));
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "the run waited {:?} for the grandchild",
        started.elapsed()
    );
}

#[test]
fn a_deadline_ends_the_job_and_the_run() {
    let started = Instant::now();

    let finished = run(
        cmd("ping -n 30 127.0.0.1"),
        &Run::new()
            .stdout(WHOLE)
            .deadline(Instant::now() + Duration::from_millis(500)),
    )
    .unwrap();

    assert_eq!(finished.exit(), Exit::TimedOut);
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "the run took {:?}",
        started.elapsed()
    );
}
