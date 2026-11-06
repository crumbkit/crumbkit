use std::io::Read as _;
use std::os::unix::process::ExitStatusExt as _;
use std::process::Stdio;
use std::time::{Duration, Instant};

use rustix::process::{Pid, getpgid, getpgrp};

use super::super::{Group, Scope, Signal};
use crate::testing::{gone, kill, pid_in, sh};

/// Starts `script` with standard output piped, in `scope`.
fn start(script: &str, scope: Scope) -> Group {
    let mut command = sh(script);
    command.stdout(Stdio::piped()).stderr(Stdio::null());
    Group::start(command, scope).unwrap()
}

/// Reads the first line that the group's child prints.
fn first_line(group: &mut Group) -> Vec<u8> {
    let mut stdout = group.take_stdout().unwrap();
    let mut line = Vec::new();
    let mut byte = [0];
    while stdout.read(&mut byte).unwrap() == 1 && byte[0] != b'\n' {
        line.push(byte[0]);
    }
    line
}

#[test]
fn a_child_leads_a_process_group_of_its_own_and_stays_unreaped_until_the_reap() {
    let mut command = sh("read line; exit 3");
    command.stdin(Stdio::piped());
    let mut group = Group::start(command, Scope::Group).unwrap();
    let pid = Pid::from_raw(group.id().try_into().unwrap()).unwrap();
    assert_eq!(getpgid(Some(pid)).ok(), Some(pid));
    assert!(!group.has_exited().unwrap());

    drop(group.take_stdin());
    group.wait_exit().unwrap();

    assert!(group.has_exited().unwrap());
    assert!(
        rustix::process::test_kill_process(pid).is_ok(),
        "the child was reaped"
    );
    assert_eq!(group.reap().unwrap().code(), Some(3));
    assert_eq!(group.reap().unwrap().code(), Some(3));
}

#[test]
fn the_reap_kills_a_background_grandchild_that_the_child_left_running() {
    let mut group = start("sleep 30 & echo $!", Scope::Group);
    let sleeper = pid_in(&first_line(&mut group));

    group.reap().unwrap();

    assert!(
        gone(sleeper),
        "the background sleep {sleeper} outlived the reap"
    );
}

#[test]
fn a_signal_reaches_the_whole_group_and_a_reaped_group_refuses_another() {
    // The background process prints its id from a new `sh`, which has no
    // trap. The forked copy of the outer shell defers a trapped SIGTERM,
    // and its exec of `sleep` then drops that signal.
    let mut group = start(
        "trap 'exit 9' TERM; sh -c 'echo $$; exec sleep 30' & wait",
        Scope::Group,
    );
    let sleeper = pid_in(&first_line(&mut group));

    group.signal(Signal::Terminate).unwrap();

    assert!(
        gone(sleeper),
        "the background sleep {sleeper} outlived the signal"
    );
    assert_eq!(group.reap().unwrap().code(), Some(9));
    assert!(group.signal(Signal::Kill).is_err());
}

#[test]
fn a_child_that_moved_to_another_group_still_gets_each_signal() {
    // perl moves itself into the group of its parent, this test's process,
    // and then waits to be killed.
    let mut group = start(
        "exec perl -e 'setpgrp(0, getpgrp(getppid())) or die; $| = 1; print qq(moved\\n); sleep 30'",
        Scope::Group,
    );
    assert_eq!(first_line(&mut group), b"moved");
    let pid = Pid::from_raw(group.id().try_into().unwrap()).unwrap();
    assert_ne!(
        getpgid(Some(pid)).ok(),
        Some(pid),
        "the child still leads its group"
    );

    group.signal(Signal::Kill).unwrap();

    assert_eq!(group.reap().unwrap().signal(), Some(9));
}

#[test]
fn a_child_that_moved_to_another_group_gets_the_signal_when_its_old_group_refuses_it() {
    // perl leaves a child that has exited, and that it never reaps, in its
    // old group, so the old group holds no live process and macOS refuses a
    // signal to it. perl then moves itself into the group of its parent,
    // this test's process, and exits 9 on SIGTERM.
    let mut group = start(
        "exec perl -e 'fork() or exit 0; select(undef, undef, undef, 0.2); \
         setpgrp(0, getpgrp(getppid())) or die; $SIG{TERM} = sub { exit 9 }; \
         $| = 1; print qq(moved\\n); sleep 30'",
        Scope::Group,
    );
    assert_eq!(first_line(&mut group), b"moved");

    let _ = group.signal(Signal::Terminate);

    let deadline = Instant::now() + Duration::from_secs(5);
    while !group.has_exited().unwrap() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let exited = group.has_exited().unwrap();
    if !exited {
        let _ = group.signal(Signal::Kill);
    }
    assert!(exited, "the moved child did not get the signal");
    assert_eq!(group.reap().unwrap().code(), Some(9));
}

#[test]
fn a_dropped_group_kills_its_child_and_its_group() {
    let mut group = start("sleep 30 & echo $!; sleep 30", Scope::Group);
    let sleeper = pid_in(&first_line(&mut group));
    let started = Instant::now();

    drop(group);

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(
        gone(sleeper),
        "the background sleep {sleeper} outlived the drop"
    );
}

#[test]
fn a_child_in_the_child_scope_stays_in_the_callers_group_and_only_it_is_signalled() {
    let mut group = start("sleep 30 & echo $!; wait", Scope::Child);
    let sleeper = pid_in(&first_line(&mut group));
    let pid = Pid::from_raw(group.id().try_into().unwrap()).unwrap();

    assert_eq!(getpgid(Some(pid)).ok(), Some(getpgrp()));
    group.signal(Signal::Kill).unwrap();
    group.reap().unwrap();

    assert!(!gone_soon(sleeper), "the background sleep was stopped");
    kill(sleeper);
}

/// Returns true when the process `pid` is gone within half a second.
fn gone_soon(pid: i32) -> bool {
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        if rustix::process::test_kill_process(Pid::from_raw(pid).unwrap()).is_err() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}
