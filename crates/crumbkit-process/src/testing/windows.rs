//! Helpers of the Windows tests. Each child is a `cmd` script, and a
//! grandchild is PowerShell, which every Windows machine has.

use std::os::windows::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Returns a command that runs `script` with `cmd`. The script is passed
/// as it is, so its quotes reach `cmd` unchanged.
pub(crate) fn cmd(script: &str) -> Command {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests build the commands that the crate runs"
    )]
    let mut cmd = Command::new("cmd");
    cmd.raw_arg("/C").raw_arg(script);
    cmd
}

/// Returns a path for the file `name` of this test process, where a
/// grandchild says that it started.
pub(crate) fn pid_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("crumbkit-process-{}-{name}", std::process::id()))
}

/// Returns a `cmd` script that starts a PowerShell grandchild in the
/// background, waits until the grandchild has printed its process id and
/// created `file`, and then prints `started`. The grandchild then sleeps
/// `seconds` and prints `woke`. The grandchild inherits the child's
/// standard output, so it holds that pipe open while it sleeps.
pub(crate) fn started_grandchild(file: &Path, seconds: u32) -> String {
    let file = file.display();
    format!(
        "start /B powershell -NoProfile -NonInteractive -Command \
         \"$PID; Set-Content -Path '{file}' -Value $PID; Start-Sleep {seconds}; 'woke'\" \
         & for /L %i in (1,1,60) do @(if not exist \"{file}\" ping -n 2 127.0.0.1 >NUL) \
         & echo started"
    )
}

/// Returns the first process id that `bytes` holds on a line of its own.
pub(crate) fn pid_in(bytes: &[u8]) -> u32 {
    String::from_utf8_lossy(bytes)
        .lines()
        .find_map(|line| line.trim().parse().ok())
        .unwrap()
}

/// Returns true when the process `pid` is running, as `tasklist` sees it.
fn alive(pid: u32) -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests ask tasklist whether a process is running"
    )]
    let listed = Command::new("tasklist")
        .raw_arg(format!("/FI \"PID eq {pid}\" /NH /FO CSV"))
        .output()
        .unwrap();
    String::from_utf8_lossy(&listed.stdout).contains(&format!("\"{pid}\""))
}

/// Returns true when the process `pid` is gone within ten seconds.
pub(crate) fn gone(pid: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while alive(pid) {
        if Instant::now() > deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    true
}

/// Removes the file at `path` that a grandchild created.
pub(crate) fn remove(path: &Path) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests remove the files that their grandchildren created"
    )]
    let _ = std::fs::remove_file(path);
}
