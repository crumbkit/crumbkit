//! The key of the usage log's hashes, which stays on the machine.
//!
//! A new key is written only while the run holds the lock on `usage.lock`,
//! and goes into place through crumbkit-durable's `write_atomically`, so
//! `usage.key` either holds 16 bytes or does not exist, even when a run ends
//! while it writes the key.

use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

use crumbkit_durable::{Mode, PathLock, write_atomically};

use super::file::{LIMITS, LOCK};
use super::platform;

#[cfg(test)]
mod tests;

/// The name of the key file beside the log.
const KEY: &str = "usage.key";

/// Returns the 16-byte key in the file `usage.key` in `dir`, and creates the
/// file on first use. A key file that does not hold exactly 16 bytes is
/// replaced with a new key. It returns `None` when the key cannot be read,
/// created or replaced, so the run writes no line.
pub(super) fn key(dir: &Path) -> Option<[u8; 16]> {
    match read(dir).ok()? {
        Some(key) => Some(key),
        None => create(dir, Instant::now() + LIMITS.wait),
    }
}

/// Reads the key file in `dir`, and returns `None` when it is missing or
/// does not hold exactly 16 bytes.
fn read(dir: &Path) -> io::Result<Option<[u8; 16]>> {
    match fs::read(dir.join(KEY)) {
        Ok(bytes) => Ok(bytes.try_into().ok()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Writes a new key to the key file in `dir` and returns it, unless another
/// run wrote a 16-byte key first, whose key it returns.
///
/// It takes the lock on `usage.lock`, waiting until `deadline`, and reads the
/// key again under the lock, so when two runs find the key missing or
/// damaged at once, one writes the key and the other reads it. It returns
/// `None` when the lock is still held at `deadline`. The lock drops when it
/// returns, before the run's append takes it again.
fn create(dir: &Path, deadline: Instant) -> Option<[u8; 16]> {
    let _lock = PathLock::acquire(&dir.join(LOCK), deadline).ok()?;
    if let Some(key) = read(dir).ok()? {
        return Some(key);
    }
    let key: [u8; 16] = platform::random_bytes()?;
    write_atomically(&dir.join(KEY), &key, Mode::Private).ok()?;
    Some(key)
}
