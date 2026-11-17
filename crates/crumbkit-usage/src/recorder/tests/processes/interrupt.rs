//! The tests of the usage log beside crumbkit's `interrupt` owner. Each test
//! runs this test binary again as the tool, in a process group of its own.

use std::os::unix::process::ExitStatusExt as _;
use std::time::{Duration, Instant};

use clap::Command;
use crumbkit::Spelling;
use crumbkit::interrupt;
use signal_hook::consts::SIGINT;

use super::{is_child, log_lines, run_child, typed};
use crate::recorder::start_under;
use crate::testing::TempDir;

#[test]
fn the_usage_log_under_start_under_writes_one_interrupted_line_on_the_second_ctrl_c() {
    if is_child() {
        let _log = start_under(interrupt::clean_up);
        let window = interrupt::install().unwrap().cooperate();
        typed(
            &Command::new("owned".to_owned()),
            &["owned".into()],
            Spelling::COLONS,
        );
        signal_hook::low_level::raise(SIGINT).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !window.requested() {
            assert!(Instant::now() < deadline, "the first Ctrl-C was not seen");
            std::thread::sleep(Duration::from_millis(5));
        }
        // The first Ctrl-C ends nothing, so the log has written no line yet.
        signal_hook::low_level::raise(SIGINT).unwrap();
        std::thread::sleep(Duration::from_secs(10));
        panic!("the second Ctrl-C did not end the tool");
    }
    let state = TempDir::new("interrupt-owner");

    let status = run_child(
        "recorder::tests::processes::interrupt::the_usage_log_under_start_under_writes_one_interrupted_line_on_the_second_ctrl_c",
        &state,
    );

    assert_eq!(
        status.signal(),
        Some(SIGINT),
        "the tool ended with {status:?}"
    );
    let lines = log_lines(&state, "owned");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["interrupted"], true);
}
