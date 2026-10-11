//! The usage log's file: its directory, its lock, its size limit and its
//! trim.

use std::fs::{DirBuilder, File, OpenOptions, TryLockError};
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::os::unix::fs::{DirBuilderExt as _, FileExt as _, OpenOptionsExt as _};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

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

/// The limits of every tool's usage log: past 10 MB the file keeps its
/// newest whole lines that fit in 5 MB, and a run waits at most 100 ms for
/// the lock. A run appends at most 1,025 bytes, so after a trim at least
/// 5 MB of new lines go in before the next trim.
pub(super) const LIMITS: Limits = Limits {
    size: 10_000_000,
    keep: 5_000_000,
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
/// `limits.size`, it first keeps only the newest whole lines that fit in
/// `limits.keep` bytes. When the file's last line has no newline, because a
/// run ended while it wrote, the new line starts on a line of its own.
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
    let mut len = file.metadata()?.len();
    if len > limits.size {
        trim(&mut file, limits.keep)?;
        len = file.metadata()?.len();
    }
    if len > 0 && !ends_with_newline(&file, len)? {
        file.write_all(b"\n")?;
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

/// Rewrites the locked `file` with only its newest whole lines that fit in
/// `keep` bytes.
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

/// Returns whether the last byte of `file`, which holds `len` bytes, is a
/// newline.
fn ends_with_newline(file: &File, len: u64) -> io::Result<bool> {
    let mut last = [0];
    file.read_exact_at(&mut last, len - 1)?;
    Ok(last[0] == b'\n')
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
