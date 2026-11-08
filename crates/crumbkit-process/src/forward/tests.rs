//! The forwarder's test runs this test binary again as the tool, in a
//! process group of its own, so the signal that the tool raises on itself
//! ends the tool and never the test runner.

use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use super::{clean_up, handed_over, install};
use crate::testing::{gone, sh};
use crate::{Limits, run};

/// The variable that tells the test's process to act as the tool.
const TOOL: &str = "CRUMBKIT_PROCESS_FORWARD_TOOL";

/// The full name of the forwarder's test, which the tool runs alone.
const TEST: &str = "forward::tests::a_signal_stops_each_live_group_runs_the_clean_ups_and_ends_the_tool_by_that_signal";

/// Returns the path of the file `name` of the tool whose process id is
/// `tool`.
fn file(tool: u32, name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("crumbkit-process-forward-{tool}-{name}"))
}

/// The tool's clean-up, which leaves a marker for the test to find.
fn mark() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tool's clean-up writes the marker that the test checks"
    )]
    let _ = std::fs::write(file(std::process::id(), "marker"), b"cleaned up\n");
}

/// Reports whether this process runs as the tool.
fn is_tool() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test tells its tool process to run through the environment"
    )]
    std::env::var_os(TOOL).is_some()
}

/// Returns the two process ids in the file at `path`, once the child has
/// written both, waiting at most ten seconds.
fn pids_in(path: &PathBuf) -> Option<(i32, i32)> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let pids: Vec<i32> = text
            .split_whitespace()
            .filter_map(|word| word.parse().ok())
            .collect();
        if text.ends_with('\n')
            && let [child, grandchild] = pids[..]
        {
            return Some((child, grandchild));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// Acts as the tool: it registers its clean-up, installs the forwarder,
/// starts a run whose child and grandchild sleep, and raises SIGTERM on
/// itself once the child has written both process ids.
fn act_as_tool() {
    clean_up(mark);
    install().unwrap();
    let pids = file(std::process::id(), "pids");
    let forwarded = file(std::process::id(), "forwarded");
    // The child's trap shows that the forwarder sent it the tool's SIGTERM
    // before the SIGKILL.
    let script = format!(
        "trap \"echo forwarded > '{}'; exit 0\" TERM; sleep 30 & echo $$ $! > '{}'; wait",
        forwarded.display(),
        pids.display()
    );
    let _runner = std::thread::spawn(move || run(sh(&script), &Limits::new()));
    assert!(pids_in(&pids).is_some(), "the child wrote no process ids");

    signal_hook::low_level::raise(signal_hook::consts::SIGTERM).unwrap();

    // The forwarder ends the tool. A tool that is still running here
    // returns, and the test then fails on its exit.
    std::thread::sleep(Duration::from_secs(10));
}

#[test]
fn a_signal_stops_each_live_group_runs_the_clean_ups_and_ends_the_tool_by_that_signal() {
    if is_tool() {
        act_as_tool();
        return;
    }
    tool_stops_its_groups(TEST);
}

#[test]
fn sigterm_after_a_hand_over_still_stops_the_groups() {
    if is_tool() {
        handed_over();
        act_as_tool();
        return;
    }
    tool_stops_its_groups("forward::tests::sigterm_after_a_hand_over_still_stops_the_groups");
}

#[test]
fn sigint_after_a_hand_over_leaves_the_tool_running() {
    if is_tool() {
        install().unwrap();
        handed_over();
        signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
        std::thread::sleep(Duration::from_millis(200));
        return;
    }

    let status = spawn_tool("forward::tests::sigint_after_a_hand_over_leaves_the_tool_running")
        .wait()
        .unwrap();

    assert_eq!(status.code(), Some(0), "the tool ended with {status:?}");
}

#[test]
fn a_second_signal_during_the_grace_changes_nothing() {
    if is_tool() {
        install().unwrap();
        let pids = file(std::process::id(), "pids");
        let script = format!(
            "trap '' TERM; sleep 30 & echo $$ $! > '{}'; wait",
            pids.display()
        );
        #[expect(
            clippy::default_trait_access,
            reason = "the test wants a run's default settings and does not depend on the name of their type"
        )]
        let _runner = std::thread::spawn(move || run(sh(&script), &Default::default()));
        assert!(pids_in(&pids).is_some(), "the child wrote no process ids");
        signal_hook::low_level::raise(signal_hook::consts::SIGTERM).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        signal_hook::low_level::raise(signal_hook::consts::SIGINT).unwrap();
        std::thread::sleep(Duration::from_secs(10));
        return;
    }

    let mut tool = spawn_tool("forward::tests::a_second_signal_during_the_grace_changes_nothing");
    let id = tool.id();
    let status = tool.wait().unwrap();

    let pids = pids_in(&file(id, "pids"));
    #[expect(
        clippy::disallowed_methods,
        reason = "the test removes the file that its tool wrote"
    )]
    let _ = std::fs::remove_file(file(id, "pids"));
    assert_eq!(status.signal(), Some(15), "the tool ended with {status:?}");
    let (child, grandchild) = pids.unwrap();
    assert!(gone(child), "the child {child} outlived the tool");
    assert!(
        gone(grandchild),
        "the grandchild {grandchild} outlived the tool"
    );
}

/// Starts this test binary as the tool that runs the test named `test`, in
/// a process group of its own.
fn spawn_tool(test: &str) -> std::process::Child {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs its own binary again as the tool"
    )]
    let mut tool = Command::new(std::env::current_exe().unwrap());
    tool.args(["--exact", test, "--nocapture"])
        .env(TOOL, "1")
        .process_group(0)
        .spawn()
        .unwrap()
}

/// Runs the tool of the test named `test`, and checks that SIGTERM ended
/// it after it had stopped its groups and run its clean-ups.
fn tool_stops_its_groups(test: &str) {
    let mut tool = spawn_tool(test);
    let id = tool.id();

    let status = tool.wait().unwrap();

    let pids = pids_in(&file(id, "pids"));
    let marker = std::fs::read_to_string(file(id, "marker"));
    let forwarded = std::fs::read_to_string(file(id, "forwarded"));
    #[expect(
        clippy::disallowed_methods,
        reason = "the test removes the files that its tool wrote"
    )]
    for name in ["pids", "marker", "forwarded"] {
        let _ = std::fs::remove_file(file(id, name));
    }
    assert_eq!(status.signal(), Some(15), "the tool ended with {status:?}");
    let (child, grandchild) = pids.unwrap();
    assert!(gone(child), "the child {child} outlived the tool");
    assert!(
        gone(grandchild),
        "the grandchild {grandchild} outlived the tool"
    );
    assert_eq!(marker.unwrap(), "cleaned up\n");
    assert_eq!(
        forwarded.unwrap(),
        "forwarded\n",
        "the child never got SIGTERM"
    );
}
