//! The usage log's file: its directory, its lock, its size limit and its
//! trim.

use std::fs::{DirBuilder, File, OpenOptions, TryLockError};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

#[cfg(test)]
mod tests;

/// The limits of the usage log's file.
pub(super) struct Limits {
    /// The size in bytes past which the next line trims the file.
    pub(super) size: u64,
    /// How many of the newest lines a trim keeps.
    pub(super) keep: usize,
    /// How long a run waits for another run's lock before it drops its line.
    pub(super) wait: Duration,
}

/// The limits of every tool's usage log: past 10 MB the file keeps its
/// newest 9,999 lines and the new one, and a run waits at most 100 ms for
/// the lock.
pub(super) const LIMITS: Limits = Limits {
    size: 10_000_000,
    keep: 9_999,
    wait: Duration::from_millis(100),
};

/// Creates `dir` and its missing parents with mode 0700, so only the user
/// can list the log and its key. A directory that exists keeps its mode.
pub(super) fn create_dir(dir: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).mode(0o700).create(dir)
}

/// Appends `line` to the file `usage.jsonl` in `dir`, which it creates with
/// mode 0600.
///
/// It holds the file's lock while it trims and appends, so runs at the same
/// time never interleave their lines. When the file is larger than
/// `limits.size`, it first keeps only the newest `limits.keep` lines.
///
/// # Errors
///
/// Returns the I/O error that stopped the line, or `TimedOut` when another
/// run held the lock for longer than `limits.wait`.
pub(super) fn append(dir: &Path, line: &[u8], limits: &Limits) -> io::Result<()> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its file, which only this function writes"
    )]
    let mut file = OpenOptions::new()
        .read(true)
        .append(true)
        .create(true)
        .mode(0o600)
        .open(dir.join("usage.jsonl"))?;
    lock(&file, limits.wait)?;
    if file.metadata()?.len() > limits.size {
        trim(&mut file, limits.keep)?;
    }
    // Closing the file when it drops releases the lock.
    file.write_all(line)
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

/// Rewrites the locked `file` with only its newest `keep` lines.
//
// RUST-10: the trim truncates the file and writes the kept lines in place,
// because a new file renamed into place would not carry the lock. A run that
// ends between the two loses the log. A trim that cannot lose lines replaces
// this if that matters.
fn trim(file: &mut File, keep: usize) -> io::Result<()> {
    let mut text = Vec::new();
    file.seek(SeekFrom::Start(0))?;
    file.read_to_end(&mut text)?;
    let kept = newest(&text, keep);
    file.set_len(0)?;
    // The file appends, so the kept lines go to the start of the empty file.
    file.write_all(kept)
}

/// Returns the newest `keep` lines of `text`, whose lines each end with a
/// newline.
fn newest(text: &[u8], keep: usize) -> &[u8] {
    let mut newlines = 0;
    for (index, byte) in text.iter().enumerate().rev() {
        if *byte == b'\n' {
            newlines += 1;
            // The first newline from the end closes the last line, so the
            // newline before the kept lines is newline `keep + 1`.
            if newlines == keep + 1 {
                return &text[index + 1..];
            }
        }
    }
    text
}
