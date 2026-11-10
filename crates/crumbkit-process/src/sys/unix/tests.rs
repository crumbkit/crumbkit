//! Tests of the Unix waiter with real children that job control stops and
//! continues.

use std::os::unix::process::ExitStatusExt as _;
use std::process::Stdio;
use std::time::{Duration, Instant};

use rustix::process::{Pid, Signal as Raw, WaitId, WaitIdOptions, kill_process, waitid};

use super::Child;
use crate::Scope;
use crate::testing::sh;

/// Returns true when a wait for an exit of the child `pid` would return at
/// once, because the kernel holds a report for it that nobody consumed.
fn reported(pid: Pid) -> bool {
    let options = WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT;
    waitid(WaitId::Pid(pid), options).unwrap().is_some()
}

/// Returns true when `check` holds within `limit`.
fn within(limit: Duration, mut check: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + limit;
    while !check() {
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    true
}

/// Returns true when `ps` shows the process `pid` stopped.
fn stopped(pid: Pid) -> bool {
    let output = sh(&format!("ps -o stat= -p {}", pid.as_raw_nonzero()))
        .output()
        .unwrap();
    output.stdout.starts_with(b"T")
}

#[test]
fn a_waiter_blocks_again_after_its_child_is_stopped_and_continued_and_still_sees_the_exit() {
    let mut command = sh("read line; exit 4");
    command.stdin(Stdio::piped());
    let mut child = Child::spawn(command, Scope::Group).unwrap();
    let pid = Pid::from_raw(child.id().try_into().unwrap()).unwrap();
    let waiter = child.waiter();
    let waiting = std::thread::spawn(move || waiter.wait());

    kill_process(pid, Raw::STOP).unwrap();
    assert!(within(Duration::from_secs(5), || stopped(pid)));
    // A report left in place would wake the waiter every time it waits.
    assert!(
        within(Duration::from_secs(1), || !reported(pid)),
        "the stop report stayed, so the waiter keeps waking"
    );
    kill_process(pid, Raw::CONT).unwrap();
    assert!(
        !waiting.is_finished(),
        "the waiter returned before the exit"
    );
    drop(child.take_stdin());

    waiting.join().unwrap().unwrap();
    assert!(child.has_exited().unwrap());
    assert_eq!(child.reap().unwrap().code(), Some(4));
}

#[test]
fn a_stopped_child_has_not_exited() {
    let mut child = Child::spawn(sh("sleep 30"), Scope::Group).unwrap();
    let pid = Pid::from_raw(child.id().try_into().unwrap()).unwrap();

    kill_process(pid, Raw::STOP).unwrap();
    assert!(within(Duration::from_secs(5), || stopped(pid)));

    assert!(!child.has_exited().unwrap());
    kill_process(pid, Raw::KILL).unwrap();
    assert_eq!(child.reap().unwrap().signal(), Some(9));
}

#[test]
fn a_running_process_in_the_group_is_live_in_it() {
    assert!(super::live_in_group("41 (sh) S 1 7 7 0", 7));
}

#[test]
fn an_exited_process_in_the_group_is_not_live() {
    assert!(!super::live_in_group("7 (sh) Z 1 7 7 0", 7));
}

#[test]
fn a_process_in_another_group_is_not_in_it() {
    assert!(!super::live_in_group("41 (sh) S 1 8 8 0", 7));
}

#[test]
fn a_name_with_spaces_and_a_parenthesis_does_not_move_the_fields() {
    assert!(super::live_in_group("41 (a) Z 9 (b) R 1 7 7 0", 7));
}

#[test]
fn a_wait_for_an_empty_group_stops_as_soon_as_nothing_is_left() {
    let mut steps = 0;
    let mut checks = 0;

    super::until_empty(
        None,
        || steps += 1,
        || {
            checks += 1;
            checks < 3
        },
    );

    assert_eq!((steps, checks), (3, 3));
}

#[test]
fn a_wait_for_an_empty_group_gives_up_when_its_time_passes() {
    let started = Instant::now();
    let give_up = started + Duration::from_millis(50);
    let mut steps = 0;

    super::until_empty(Some(give_up), || steps += 1, || true);

    assert!(Instant::now() >= give_up);
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(steps > 1, "the wait stepped once and stopped");
}
