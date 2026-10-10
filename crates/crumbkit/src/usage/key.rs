//! The key of the usage log's hashes, which stays on the machine.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;

use super::random;

#[cfg(test)]
mod tests;

/// Returns the 16-byte key in the file `usage.key` in `dir`, and creates the
/// file on first use. It returns `None` when the key cannot be read or
/// created, and when the file does not hold exactly 16 bytes, so the run
/// writes no line.
pub(super) fn key(dir: &Path) -> Option<[u8; 16]> {
    let path = dir.join("usage.key");
    match fs::read(&path) {
        Ok(bytes) => bytes.try_into().ok(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => create(&path),
        Err(_) => None,
    }
}

/// Creates the key file at `path` with 16 bytes from `/dev/urandom` and mode
/// 0600, and returns the key. The file is created with `create_new`, so when
/// two first runs race, one creates it and the other reads its key.
//
// RUST-10: the winner creates the file and then writes the key, so a run
// that reads the file between the two finds it empty and writes no line. It
// misses that one line. Writing the key to a temporary file and linking it
// into place replaces this if lines go missing.
fn create(path: &Path) -> Option<[u8; 16]> {
    let key: [u8; 16] = random::bytes()?;
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its key file, which only this function creates"
    )]
    let created = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path);
    match created {
        Ok(mut file) => {
            if file.write_all(&key).is_ok() {
                return Some(key);
            }
            // A key file left short, as on a full disk, would stop every
            // later line, so the next run starts again without it.
            #[expect(
                clippy::disallowed_methods,
                reason = "the usage log owns its key file, and removes one that it could not finish"
            )]
            let _ = fs::remove_file(path);
            None
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            fs::read(path).ok()?.try_into().ok()
        }
        Err(_) => None,
    }
}
