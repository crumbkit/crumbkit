//! Creating a file, the directory sync that Windows skips, and opening a
//! lock file that no process can remove or replace while it is open.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt as _;
use std::path::Path;

use crate::Mode;

/// Creates the new file `path` for writing, and fails with
/// `AlreadyExists` when anything, including a symbolic link, is at `path`.
//
// RUST-10: `mode` is ignored, and the file inherits the access list of its
// directory, so a `Mode::Private` file is as readable as its directory's
// other files. A private file on Windows needs an explicit access list,
// which std cannot set; a crate such as `windows-acl` would replace this
// open once a caller needs one.
pub(crate) fn open_new(path: &Path, _mode: Mode) -> io::Result<File> {
    #[expect(
        clippy::disallowed_methods,
        reason = "crumbkit-durable owns the creation of its temporary file"
    )]
    OpenOptions::new().write(true).create_new(true).open(path)
}

/// Does nothing and returns `Ok`.
//
// RUST-10: std cannot open a directory on Windows, because that needs
// `FILE_FLAG_BACKUP_SEMANTICS`, so the rename is not followed by a
// directory flush. NTFS journals the rename itself, so after a power loss
// the target holds the old or the new bytes. A flush through the
// `windows-sys` crate would replace this if a file system without that
// journal had to be supported.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Unix form of this function returns the sync's error"
)]
pub(crate) fn sync_directory(_directory: &Path) -> io::Result<()> {
    Ok(())
}

/// `FILE_SHARE_READ | FILE_SHARE_WRITE` from the Windows API, written out
/// because std names no share mode.
const SHARE_READ_AND_WRITE: u32 = 0x1 | 0x2;

/// Opens the lock file `path` for writing, creating it when it is absent.
///
/// The share mode leaves out `FILE_SHARE_DELETE`, so while any process
/// holds the file open, no process can remove it or rename another file
/// over it. The open never truncates the file.
//
// RUST-10: the file inherits its directory's access list, as `open_new`
// says, and a symbolic link at `path` is followed. A lock file that must
// refuse a link would open with `FILE_FLAG_OPEN_REPARSE_POINT` through
// `custom_flags` and check the result's attributes.
pub(crate) fn open_lock(path: &Path) -> io::Result<File> {
    #[expect(
        clippy::disallowed_methods,
        reason = "crumbkit-durable owns the creation of a lock file"
    )]
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(SHARE_READ_AND_WRITE)
        .open(path)
}

/// The user a lock file's owner is compared with, which Windows never
/// checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Caller;

impl Caller {
    /// The one caller.
    pub(crate) fn current() -> Self {
        Self
    }
}

/// Accepts every lock file, because Windows lock files are not checked for
/// an owner.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Unix form of this function returns the owner mismatch"
)]
pub(crate) fn check_owner(_file: &File, _path: &Path, _caller: Caller) -> io::Result<()> {
    Ok(())
}

/// The identity of a lock file, which on Windows never needs a check,
/// because the open lock file cannot be removed or replaced. Every
/// identity equals every other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Identity;

/// Returns the one identity, without a system call.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Unix form of this function returns the fstat's error"
)]
pub(crate) fn file_identity(_file: &File) -> io::Result<Identity> {
    Ok(Identity)
}

/// Returns the one identity, without a system call.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the Unix form of this function returns the lstat's error"
)]
pub(crate) fn path_identity(_path: &Path) -> io::Result<Option<Identity>> {
    Ok(Some(Identity))
}
