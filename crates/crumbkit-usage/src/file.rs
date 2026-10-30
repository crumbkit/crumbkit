//! The usage log's file: its directory, its lock file, its size limit and
//! its trim.

use std::fs::{self, DirBuilder, File, OpenOptions, TryLockError};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use super::platform;

#[cfg(test)]
mod tests;

/// The limits of the usage log's file.
pub(super) struct Limits {
    /// The size in bytes past which the next line trims the file.
    pub(super) size: u64,
    /// The most bytes that a trim keeps, as the newest whole lines that fit.
    pub(super) keep: usize,
    /// How long a run waits for another run's lock before it drops its line.
    pub(super) wait: Duration,
}

/// The name of the log file in the tool's directory under the XDG state
/// directory.
pub(super) const LOG: &str = "usage.jsonl";

/// Returns the usage log of the tool named `tool`: `usage.jsonl` in the
/// tool's directory under the state directory, or `None` when there is no
/// state directory. [`start`](crate::start) writes the log there, and
/// `self usage` and `self figures list` read it there.
#[must_use]
pub fn log_path(tool: &str) -> Option<PathBuf> {
    Some(platform::state_home()?.join(tool).join(LOG))
}

/// The limits of every tool's usage log: past 10 MB the file keeps its
/// newest whole lines that fit in 5 MB, and a run waits at most 100 ms for
/// the lock. A run appends at most 1,025 bytes, so after a trim at least
/// 5 MB of new lines go in before the next trim.
pub(super) const LIMITS: Limits = Limits {
    size: 10_000_000,
    keep: 5_000_000,
    wait: Duration::from_millis(100),
};

/// Creates `dir` and its missing parents so that only the user can list
/// the log and its key, with mode 0700 on Unix. A directory that exists
/// keeps its permissions.
pub(super) fn create_dir(dir: &Path) -> io::Result<()> {
    platform::private_dir(DirBuilder::new().recursive(true)).create(dir)
}

/// Appends `line` to the file `usage.jsonl` in `dir`, which it creates so
/// that only the user can read it, with mode 0600 on Unix.
///
/// It holds the lock on the file `usage.lock` in `dir`, which it creates in
/// the same way, while it trims and appends, so runs at the same time never
/// interleave their lines. The lock has a file of its own because
/// a trim renames a new log over `usage.jsonl`, and a lock on the old log
/// would no longer guard the new one. When the log is larger than
/// `limits.size`, it first keeps only the newest whole lines that fit in
/// `limits.keep` bytes. When the log's last line has no newline, because a
/// run ended while it wrote, the new line starts on a line of its own.
///
/// # Errors
///
/// Returns the I/O error that stopped the line, or `TimedOut` when another
/// run held the lock for longer than `limits.wait`.
pub(super) fn append(dir: &Path, line: &[u8], limits: &Limits) -> io::Result<()> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its lock file, which only this function creates"
    )]
    let lock_file =
        platform::private_file(OpenOptions::new().write(true).create(true).truncate(false))
            .open(dir.join("usage.lock"))?;
    lock(&lock_file, limits.wait)?;
    let mut file = open_log(dir)?;
    let mut len = file.metadata()?.len();
    if len > limits.size {
        trim(dir, file, limits.keep)?;
        file = open_log(dir)?;
        len = file.metadata()?.len();
    }
    if len > 0 && !ends_with_newline(&file, len)? {
        file.write_all(b"\n")?;
    }
    file.write_all(line)?;
    // Closing the lock file releases the lock, after the line is written.
    drop(lock_file);
    Ok(())
}

/// Opens the log `usage.jsonl` in `dir` to read and append, and creates it
/// so that only the user can read it.
fn open_log(dir: &Path) -> io::Result<File> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its file, which only this module writes"
    )]
    platform::private_file(OpenOptions::new().read(true).append(true).create(true))
        .open(dir.join(LOG))
}

/// Takes the lock on `file`, and retries every 5 ms for at most `wait`.
fn lock(file: &File, wait: Duration) -> io::Result<()> {
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) if started.elapsed() < wait => {
                thread::sleep(Duration::from_millis(5));
            }
            Err(TryLockError::WouldBlock) => return Err(io::ErrorKind::TimedOut.into()),
            Err(TryLockError::Error(error)) => return Err(error),
        }
    }
}

/// Replaces the log `file` in `dir` with a new log that holds only its
/// newest whole lines that fit in `keep` bytes. The kept lines go to a
/// temporary file in `dir`, which is synced and then renamed over
/// `usage.jsonl`, so a run that ends during the trim leaves either the old
/// log or the new one. The caller holds the lock. `file` closes before the
/// rename, because Windows can refuse to rename over a file that is open.
fn trim(dir: &Path, mut file: File, keep: usize) -> io::Result<()> {
    let mut text = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut text)?;
    drop(file);
    let temporary = dir.join(format!("usage.jsonl.{}.tmp", std::process::id()));
    let replaced = write_trimmed(&temporary, newest(&text, keep)).and_then(|()| {
        #[expect(
            clippy::disallowed_methods,
            reason = "the usage log owns its file, which a trim replaces"
        )]
        fs::rename(&temporary, dir.join(LOG))
    });
    if replaced.is_err() {
        #[expect(
            clippy::disallowed_methods,
            reason = "the usage log owns its temporary file, and removes one that a trim could not finish"
        )]
        let _ = fs::remove_file(&temporary);
    }
    replaced
}

/// Writes `kept` to the file at `temporary`, which only the user can read,
/// and syncs it, so the bytes reach the disk before the file replaces the log. A file left by
/// an earlier process with the same id is overwritten.
fn write_trimmed(temporary: &Path, kept: &[u8]) -> io::Result<()> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its file, which a trim writes to a temporary file first"
    )]
    let mut file =
        platform::private_file(OpenOptions::new().write(true).create(true).truncate(true))
            .open(temporary)?;
    file.write_all(kept)?;
    file.sync_all()
}

/// Returns whether the last byte of `file`, which holds `len` bytes, is a
/// newline.
fn ends_with_newline(file: &File, len: u64) -> io::Result<bool> {
    Ok(platform::read_byte_at(file, len - 1)? == b'\n')
}

/// Returns the newest whole lines of `text` that fit in `keep` bytes. A last
/// line without a newline is a write that ended early, and it is dropped.
fn newest(text: &[u8], keep: usize) -> &[u8] {
    let end = text
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |index| index + 1);
    let whole = &text[..end];
    let Some(earliest) = end.checked_sub(keep).filter(|earliest| *earliest > 0) else {
        return whole;
    };
    // A kept line starts at `earliest` or later, so the kept lines start
    // after the first newline at `earliest - 1` or later. `whole` ends with
    // a newline, so there is one.
    whole[earliest - 1..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(&[], |offset| &whole[earliest + offset..])
}
