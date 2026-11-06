use std::process::Stdio;
use std::time::{Duration, Instant};

use super::super::{Group, Scope};
use crate::testing::{cmd, gone, pid_file, remove, started_grandchild};

#[test]
fn a_dropped_group_ends_its_job_and_the_grandchild_in_it() {
    let file = pid_file("dropped-group");
    let mut command = cmd(&format!(
        "{} & ping -n 120 127.0.0.1 >NUL",
        started_grandchild(&file, 120)
    ));
    command.stdout(Stdio::null()).stderr(Stdio::null());
    let group = Group::start(command, Scope::Group).unwrap();
    let grandchild = written_pid(&file);

    drop(group);

    remove(&file);
    assert!(
        gone(grandchild),
        "the grandchild {grandchild} outlived the drop"
    );
}

/// Returns the process id that a grandchild wrote to `file`, waiting at
/// most a minute for it.
fn written_pid(file: &std::path::Path) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if let Ok(pid) = text.trim().trim_start_matches('\u{feff}').parse() {
            return pid;
        }
        assert!(Instant::now() < deadline, "the grandchild wrote no id");
        std::thread::sleep(Duration::from_millis(100));
    }
}
