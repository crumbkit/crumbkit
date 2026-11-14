//! Creating a file, and the directory sync that Windows skips.

use std::fs::{File, OpenOptions};
use std::io;
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
