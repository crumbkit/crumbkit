//! Tests of the hook around the start of a run's child.

use std::fs::File;
use std::io;
use std::os::fd::AsRawFd as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};

use super::{after, kept, run_sh};
use crate::testing::{gone, pid_in, sh};
use crate::{Cancel, Outcome, RunError, RunOptions, Status, run};

/// A path in the system's temporary directory that this test alone uses.
fn scratch(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "crumbkit-around-spawn-{}-{name}",
        std::process::id()
    ))
}

/// Removes the file a test's child wrote.
fn remove(path: &Path) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test removes the file that its child wrote"
    )]
    std::fs::remove_file(path).unwrap();
}

/// Returns the process id that the child wrote to `path`, once it wrote it.
fn pid_from(path: &Path) -> i32 {
    let deadline = after(5.0);
    loop {
        let written = std::fs::read(path).unwrap_or_default();
        if written.ends_with(b"\n") {
            return pid_in(&written);
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the child wrote no pid"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A script that writes its process id to `path` and then runs for 30 seconds.
fn writes_pid_then_sleeps(path: &Path) -> String {
    format!("echo $$ > {}; exec sleep 30", path.display())
}

#[test]
fn the_hook_runs_its_steps_before_and_after_the_child_starts() {
    let steps = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&steps);
    let options = kept().around_spawn(move |spawn| {
        seen.lock().unwrap().push("before");
        let spawned = spawn.start();
        seen.lock().unwrap().push("after");
        spawned
    });

    let finished = run_sh("echo started", &options);

    assert_eq!(*steps.lock().unwrap(), ["before", "after"]);
    assert_eq!(finished.outcome(), Outcome::Ended(Status::Code(0)));
    assert_eq!(finished.stdout().bytes(), b"started\n");
}

#[test]
fn a_descriptor_the_hook_makes_inheritable_reaches_the_child_and_is_closed_on_exec_again() {
    let file = File::open("/dev/null").unwrap();
    let number = file.as_raw_fd();
    assert!(fcntl_getfd(&file).unwrap().contains(FdFlags::CLOEXEC));
    let shared = Arc::new(file);
    let held = Arc::clone(&shared);
    let options = kept().around_spawn(move |spawn| {
        fcntl_setfd(&*held, FdFlags::empty())?;
        let spawned = spawn.start();
        fcntl_setfd(&*held, FdFlags::CLOEXEC)?;
        spawned
    });

    let open = run_sh(&format!("test -e /dev/fd/{number}"), &options);
    let closed = run_sh(&format!("test -e /dev/fd/{number}"), &kept());

    assert_eq!(open.outcome(), Outcome::Ended(Status::Code(0)));
    assert_eq!(closed.outcome(), Outcome::Ended(Status::Code(1)));
    assert!(fcntl_getfd(&*shared).unwrap().contains(FdFlags::CLOEXEC));
}

#[test]
fn a_hook_that_fails_before_the_start_starts_no_child() {
    let marker = scratch("never-started");
    let options = kept().around_spawn(|_spawn| Err(io::Error::other("the gate is closed")));

    let refused = run(sh(&format!("touch {}", marker.display())), &options).unwrap_err();

    let RunError::Start(error) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(error.to_string(), "the gate is closed");
    std::thread::sleep(Duration::from_millis(100));
    assert!(!marker.exists());
}

#[test]
fn a_hook_that_fails_after_the_start_stops_and_reaps_the_child() {
    let path = scratch("failed-after");
    let script = writes_pid_then_sleeps(&path);
    let wrote = path.clone();
    let options = kept().around_spawn(move |spawn| {
        let _spawned = spawn.start()?;
        let pid = pid_from(&wrote);
        Err(io::Error::other(format!("restore failed after {pid}")))
    });

    let refused = run(sh(&script), &options).unwrap_err();

    let RunError::Start(error) = refused else {
        panic!("{refused:?}");
    };
    let pid = error
        .to_string()
        .rsplit(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(gone(pid));
    remove(&path);
}

#[test]
fn a_hook_that_panics_after_the_start_stops_and_reaps_the_child() {
    let path = scratch("panicked-after");
    let script = writes_pid_then_sleeps(&path);
    let wrote = path.clone();
    let options = kept().around_spawn(move |spawn| {
        let _spawned = spawn.start()?;
        let pid = pid_from(&wrote);
        std::panic::panic_any(pid)
    });

    let unwound = catch_unwind(AssertUnwindSafe(|| run(sh(&script), &options))).unwrap_err();

    assert!(gone(*unwound.downcast::<i32>().unwrap()));
    remove(&path);
}

#[test]
fn a_run_that_is_already_cancelled_never_calls_the_hook() {
    let cancel = Cancel::new();
    cancel.cancel();
    let called = Arc::new(Mutex::new(false));
    let seen = Arc::clone(&called);
    let options = kept().cancel(&cancel).around_spawn(move |spawn| {
        *seen.lock().unwrap() = true;
        spawn.start()
    });

    let finished = run_sh("exit 0", &options);

    assert_eq!(finished.status(), None);
    assert!(!*called.lock().unwrap());
}

#[test]
fn a_clone_of_the_settings_keeps_the_hook() {
    let calls = Arc::new(Mutex::new(0));
    let seen = Arc::clone(&calls);
    let options = kept().around_spawn(move |spawn| {
        *seen.lock().unwrap() += 1;
        spawn.start()
    });

    run_sh("exit 0", &options);
    run_sh("exit 0", &options.clone());

    assert_eq!(*calls.lock().unwrap(), 2);
}

#[test]
fn the_time_to_the_first_byte_leaves_out_the_time_the_hook_waited() {
    let wait = Duration::from_secs(1);
    let options = kept().around_spawn(move |spawn| {
        std::thread::sleep(wait);
        spawn.start()
    });
    let began = std::time::Instant::now();

    let finished = run_sh("echo started", &options);

    assert!(finished.first_stdout().unwrap() + wait <= began.elapsed());
}

#[test]
fn settings_with_a_hook_can_be_debug_printed() {
    let options = RunOptions::new().around_spawn(crate::Spawn::start);

    assert!(format!("{options:?}").contains("around_spawn"));
}
