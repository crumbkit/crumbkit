//! The `interrupt` owner, which each test runs in a copy of this test
//! binary that raises signals in itself, in a process group of its own so a
//! signal never reaches the test runner.
#![cfg(all(unix, feature = "interrupt"))]
#![expect(
    clippy::unwrap_used,
    clippy::panic,
    clippy::disallowed_methods,
    reason = "a test child unwraps its setup, panics when a signal fails to end it, and writes to the real standard streams that its parent reads"
)]

use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use clap::Command as Clap;
use crumbkit::interrupt::{self, Interrupts};
use crumbkit::{DryRun, Run, Sink, Tool, ViewFlags, codes, events};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::low_level::raise;

/// Tells a test running as the child to do the work.
const CHILD: &str = "CRUMBKIT_INTERRUPT_CHILD";

/// Reports whether this process runs as a signal test's child.
fn is_child() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "a signal test tells its child process to run through the environment"
    )]
    std::env::var_os(CHILD).is_some()
}

/// Runs the test `name` again as the child and returns how it ended.
fn run_child(name: &str) -> Output {
    #[expect(
        clippy::disallowed_methods,
        reason = "a signal test runs this test binary again as the child that receives the signal"
    )]
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", name, "--nocapture", "--test-threads", "1"])
        .env(CHILD, "1")
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    child.output().unwrap()
}

/// Waits for `ready` to hold, and panics after five seconds.
fn wait_for(what: &str, ready: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !ready() {
        assert!(Instant::now() < deadline, "{what} did not happen");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Parks the child until a signal ends it.
fn wait_to_be_ended() -> ! {
    std::thread::sleep(Duration::from_secs(10));
    panic!("the signal did not end the process");
}

/// Marks the clean-up that ran, on standard output.
fn say_clean_up_ran() {
    #[expect(
        clippy::print_stdout,
        reason = "the child tells its test that the clean-up ran"
    )]
    {
        println!("clean-up ran");
    }
}

#[test]
fn the_first_ctrl_c_inside_a_window_keeps_the_process_running_and_check_returns_interrupted() {
    if is_child() {
        let window = interrupt::install().unwrap().cooperate();
        assert!(window.check().is_ok());
        raise(SIGINT).unwrap();
        wait_for("the request", || window.requested());
        assert_eq!(window.check(), Err(interrupt::Interrupted));
        return;
    }
    let out = run_child(
        "the_first_ctrl_c_inside_a_window_keeps_the_process_running_and_check_returns_interrupted",
    );
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn an_interrupted_command_prints_the_interrupted_failure_and_exits_130() {
    if is_child() {
        let window = interrupt::install().unwrap().cooperate();
        raise(SIGINT).unwrap();
        wait_for("the request", || window.requested());
        let failure = window.check().unwrap_err();
        let tool = Tool::new();
        let mut run = Run::new(
            ViewFlags::new(),
            DryRun::default(),
            Clap::new("tool"),
            &tool,
            ["tool"],
        )
        .unwrap();
        let exit = run.failure_exit(codes::RUN_INTERRUPTED);
        let _ = run.fail(
            &mut Sink::new(std::io::stdout(), std::io::stderr()),
            &failure,
        );
        std::process::exit(i32::from(exit));
    }
    let out = run_child("an_interrupted_command_prints_the_interrupted_failure_and_exits_130");
    assert_eq!(out.status.code(), Some(130), "{out:?}");
    let printed = String::from_utf8_lossy(&out.stderr);
    assert!(
        printed.contains("code:  tool::run::interrupted"),
        "{printed}"
    );
}

#[test]
fn a_second_ctrl_c_inside_a_window_ends_the_process_as_sigint_does() {
    if is_child() {
        interrupt::clean_up(say_clean_up_ran);
        let window = interrupt::install().unwrap().cooperate();
        raise(SIGINT).unwrap();
        wait_for("the request", || window.requested());
        raise(SIGINT).unwrap();
        wait_to_be_ended();
    }
    let out = run_child("a_second_ctrl_c_inside_a_window_ends_the_process_as_sigint_does");
    assert_eq!(out.status.signal(), Some(SIGINT), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("clean-up ran"));
}

#[test]
fn a_ctrl_c_outside_a_window_runs_the_clean_ups_and_ends_the_process() {
    if is_child() {
        interrupt::clean_up(say_clean_up_ran);
        let _interrupts: Interrupts = interrupt::install().unwrap();
        raise(SIGINT).unwrap();
        wait_to_be_ended();
    }
    let out = run_child("a_ctrl_c_outside_a_window_runs_the_clean_ups_and_ends_the_process");
    assert_eq!(out.status.signal(), Some(SIGINT), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("clean-up ran"));
}

#[test]
fn sigterm_runs_the_clean_ups_and_ends_the_process() {
    if is_child() {
        interrupt::clean_up(say_clean_up_ran);
        let _window = interrupt::install().unwrap().cooperate();
        raise(SIGTERM).unwrap();
        wait_to_be_ended();
    }
    let out = run_child("sigterm_runs_the_clean_ups_and_ends_the_process");
    assert_eq!(out.status.signal(), Some(SIGTERM), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("clean-up ran"));
}

#[test]
fn a_clean_up_that_panics_does_not_stop_the_next_one() {
    fn panics() {
        panic!("a clean-up failed");
    }
    if is_child() {
        interrupt::clean_up(panics);
        interrupt::clean_up(say_clean_up_ran);
        let _interrupts = interrupt::install().unwrap();
        raise(SIGTERM).unwrap();
        wait_to_be_ended();
    }
    let out = run_child("a_clean_up_that_panics_does_not_stop_the_next_one");
    assert_eq!(out.status.signal(), Some(SIGTERM), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("clean-up ran"));
}

#[test]
fn handles_interrupt_is_true_once_the_owner_is_installed() {
    if is_child() {
        assert!(!events::handles_interrupt());
        let _interrupts = interrupt::install().unwrap();
        assert!(events::handles_interrupt());
        return;
    }
    let out = run_child("handles_interrupt_is_true_once_the_owner_is_installed");
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn the_owner_drops_sigint_while_a_child_holds_the_terminal() {
    if is_child() {
        let window = interrupt::install().unwrap().cooperate();
        events::handed_over();
        raise(SIGINT).unwrap();
        raise(SIGINT).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        assert!(!window.requested());
        return;
    }
    let out = run_child("the_owner_drops_sigint_while_a_child_holds_the_terminal");
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn a_request_survives_the_drop_of_another_open_guard() {
    if is_child() {
        let first = interrupt::install().unwrap().cooperate();
        let second = interrupt::install().unwrap().cooperate();
        drop(first);
        raise(SIGINT).unwrap();
        wait_for("the request", || second.requested());
        assert_eq!(second.check(), Err(interrupt::Interrupted));
        return;
    }
    let out = run_child("a_request_survives_the_drop_of_another_open_guard");
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn a_second_window_keeps_a_request_that_the_first_has_not_read() {
    if is_child() {
        let first = interrupt::install().unwrap().cooperate();
        raise(SIGINT).unwrap();
        wait_for("the request", || first.requested());
        let second = interrupt::install().unwrap().cooperate();
        assert_eq!(second.check(), Err(interrupt::Interrupted));
        assert_eq!(first.check(), Err(interrupt::Interrupted));
        return;
    }
    let out = run_child("a_second_window_keeps_a_request_that_the_first_has_not_read");
    assert!(out.status.success(), "{out:?}");
}

#[test]
fn a_ctrl_c_after_the_last_guard_dropped_ends_the_process() {
    if is_child() {
        interrupt::clean_up(say_clean_up_ran);
        let first = interrupt::install().unwrap().cooperate();
        let second = interrupt::install().unwrap().cooperate();
        drop(first);
        drop(second);
        raise(SIGINT).unwrap();
        wait_to_be_ended();
    }
    let out = run_child("a_ctrl_c_after_the_last_guard_dropped_ends_the_process");
    assert_eq!(out.status.signal(), Some(SIGINT), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("clean-up ran"));
}

#[test]
fn a_guard_that_unwinds_in_a_panic_still_leaves_the_other_guard_open() {
    if is_child() {
        let first = interrupt::install().unwrap().cooperate();
        let unwound = std::panic::catch_unwind(|| {
            let _second = interrupt::install().unwrap().cooperate();
            panic!("a command failed inside its window");
        });
        assert!(unwound.is_err());
        raise(SIGINT).unwrap();
        wait_for("the request", || first.requested());
        assert_eq!(first.check(), Err(interrupt::Interrupted));
        return;
    }
    let out = run_child("a_guard_that_unwinds_in_a_panic_still_leaves_the_other_guard_open");
    assert!(out.status.success(), "{out:?}");
}
