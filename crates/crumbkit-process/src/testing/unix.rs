//! Helpers of the Unix tests. Each child is a POSIX `sh` script, so the
//! tests run on every Unix machine without a test program of their own.

use std::process::Command;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal, kill_process, test_kill_process};

/// Returns a command that runs `script` with `sh`.
pub(crate) fn sh(script: &str) -> Command {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests build the commands that the crate runs"
    )]
    let mut sh = Command::new("sh");
    sh.args(["-c", script]);
    sh
}

/// Returns the first process id that `bytes` holds on a line of its own.
pub(crate) fn pid_in(bytes: &[u8]) -> i32 {
    String::from_utf8_lossy(bytes)
        .lines()
        .find_map(|line| line.trim().parse().ok())
        .unwrap()
}

/// Returns true when the process `pid` is gone within five seconds. A
/// process that has exited and that its new parent has not yet reaped
/// still answers a signal, so the check waits for the reap.
pub(crate) fn gone(pid: i32) -> bool {
    let pid = Pid::from_raw(pid).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while test_kill_process(pid).is_ok() {
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

/// Kills the process `pid`, which a test left running on purpose.
pub(crate) fn kill(pid: i32) {
    let _ = kill_process(Pid::from_raw(pid).unwrap(), Signal::KILL);
}
