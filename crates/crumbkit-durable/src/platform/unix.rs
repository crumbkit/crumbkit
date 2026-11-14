//! Creating a file with a mode, and syncing a directory, on Unix.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;

use crate::Mode;

/// Creates the new file `path` for writing with the permission bits of
/// `mode`, and fails with `AlreadyExists` when anything, including a
/// symbolic link, is at `path`.
///
/// `create_new` is `O_CREAT | O_EXCL`, which never follows a symbolic link,
/// so the open needs no `O_NOFOLLOW`. std opens with `O_CLOEXEC` and retries
/// on `EINTR`.
pub(crate) fn open_new(path: &Path, mode: Mode) -> io::Result<File> {
    let bits = match mode {
        Mode::Private => 0o600,
        Mode::Umask => 0o666,
    };
    #[expect(
        clippy::disallowed_methods,
        reason = "crumbkit-durable owns the creation of its temporary file"
    )]
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(bits)
        .open(path)
}

/// Syncs the entries of `directory`, so a rename in it survives a power
/// loss. `sync_all` is `fsync`, and `F_FULLFSYNC` on macOS.
pub(crate) fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}
