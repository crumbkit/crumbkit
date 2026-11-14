//! Creating a file with a mode, syncing a directory, and opening and
//! identifying a lock file, on Unix.

use std::fs::{self, File, Metadata, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _};
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

/// Opens the lock file `path` for writing, creating it with `0600` when it
/// is absent, and fails with `ELOOP` when `path` is a symbolic link.
///
/// The open never truncates the file. `O_NOFOLLOW` comes from rustix,
/// because std names no open flag. std opens with `O_CLOEXEC` and retries
/// on `EINTR`.
pub(crate) fn open_lock(path: &Path) -> io::Result<File> {
    #[expect(
        clippy::disallowed_methods,
        reason = "crumbkit-durable owns the creation of a lock file"
    )]
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits().cast_signed())
        .open(path)
}

/// The device and inode of a file, which name it whatever path reaches it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    device: u64,
    inode: u64,
}

impl Identity {
    fn of(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        }
    }
}

/// Returns the identity of the open `file`, from `fstat`.
pub(crate) fn file_identity(file: &File) -> io::Result<Identity> {
    Ok(Identity::of(&file.metadata()?))
}

/// Returns the identity of the file at `path`, from `lstat`, which never
/// follows a symbolic link, or `None` when nothing is at `path`.
pub(crate) fn path_identity(path: &Path) -> io::Result<Option<Identity>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(Identity::of(&metadata))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
