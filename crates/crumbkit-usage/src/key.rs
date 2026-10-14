//! The key of the usage log's hashes, which stays on the machine.
//!
//! A new key goes to a temporary file first and then into place under the
//! name `usage.key` in one step, so `usage.key` either holds 16 bytes or
//! does not exist, even when a run ends while it writes the key.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use super::platform;

#[cfg(test)]
mod tests;

/// Returns the 16-byte key in the file `usage.key` in `dir`, and creates the
/// file on first use. A key file that does not hold exactly 16 bytes is
/// replaced with a new key. It returns `None` when the key cannot be read,
/// created or replaced, so the run writes no line.
pub(super) fn key(dir: &Path) -> Option<[u8; 16]> {
    match fs::read(dir.join("usage.key")) {
        Ok(bytes) => match bytes.try_into() {
            Ok(key) => Some(key),
            Err(_) => replace(dir),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => create(dir),
        Err(_) => None,
    }
}

/// Creates the key file in `dir` with a new key, and returns the key. The
/// key goes into place through `hard_link`, which fails when the name
/// exists, so when two first runs race, one creates the file and the other
/// reads its key.
fn create(dir: &Path) -> Option<[u8; 16]> {
    let (key, temporary) = write_temporary(dir)?;
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its key file, which only this module creates"
    )]
    let linked = fs::hard_link(&temporary, dir.join("usage.key"));
    remove(&temporary);
    match linked {
        Ok(()) => Some(key),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            fs::read(dir.join("usage.key")).ok()?.try_into().ok()
        }
        Err(_) => None,
    }
}

/// Replaces the key file in `dir`, which holds the wrong number of bytes,
/// with a new key, and returns the key.
//
// RUST-10: two runs that find the same damaged key at the same time each
// rename a key of their own into place. The run whose key is replaced
// writes its one line with a key that no file keeps, so that line's
// `session` and `args` hashes match no other line. A lock around the
// replacement prevents this if it matters. A damaged key is rare, because
// `create` never leaves one.
fn replace(dir: &Path) -> Option<[u8; 16]> {
    let (key, temporary) = write_temporary(dir)?;
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its key file, which only this module replaces"
    )]
    let renamed = fs::rename(&temporary, dir.join("usage.key"));
    if renamed.is_err() {
        remove(&temporary);
        return None;
    }
    Some(key)
}

/// Writes 16 random bytes to a temporary file in `dir` that only the user
/// can read, and returns the key and the file's path. The file's name holds
/// the process id, so runs at the same time never share it, and a file left
/// by an earlier process with the same id is overwritten.
fn write_temporary(dir: &Path) -> Option<([u8; 16], PathBuf)> {
    let key: [u8; 16] = platform::random_bytes()?;
    let temporary = dir.join(format!("usage.key.{}.tmp", std::process::id()));
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its key file, which this module writes to a temporary file first"
    )]
    let written =
        platform::private_file(OpenOptions::new().write(true).create(true).truncate(true))
            .open(&temporary)
            .and_then(|mut file| {
                file.write_all(&key)?;
                // The bytes reach the disk before the name does, so a machine
                // that stops never leaves a named key without its bytes.
                file.sync_all()
            });
    if written.is_err() {
        remove(&temporary);
        return None;
    }
    Some((key, temporary))
}

/// Removes the temporary key file at `path`. A failure leaves an unused file
/// behind, which the next run of a process with the same id overwrites.
fn remove(path: &Path) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its temporary key file, and removes it once the key is in place"
    )]
    let _ = fs::remove_file(path);
}
