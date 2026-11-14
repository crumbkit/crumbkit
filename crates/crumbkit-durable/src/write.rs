//! Replacing a file through a new temporary file, a sync and a rename.

use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::platform;

#[cfg(test)]
mod tests;

/// The permission bits that [`write_atomically`] gives the new file on Unix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Readable and writable by the owner alone, `0600`, for a file such as
    /// a key.
    Private,
    /// `0666` less the process's umask, as a file that `File::create` makes.
    Umask,
}

/// The step of [`write_atomically`] that failed.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteStep {
    /// Creating the temporary file. The target is unchanged.
    Create,
    /// Writing the bytes to the temporary file. The target is unchanged.
    Write,
    /// Syncing the temporary file to disk. The target is unchanged.
    Sync,
    /// Renaming the temporary file over the target. The target is
    /// unchanged.
    Rename,
    /// Syncing the target's directory after the rename. The target already
    /// holds the new bytes, and after a power loss it may hold the old
    /// bytes, never part of either.
    SyncDirectory,
}

/// Why [`write_atomically`] failed: the step, the file it was asked to
/// replace and the operating system's error.
///
/// Its `Display` names the step and the file in one sentence, and
/// `source` returns the operating system's error.
#[non_exhaustive]
#[derive(Debug)]
pub struct WriteError {
    /// The step that failed.
    pub step: WriteStep,
    /// The file that [`write_atomically`] was asked to replace.
    pub path: PathBuf,
    /// The operating system's error.
    pub source: io::Error,
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.path.display();
        match self.step {
            WriteStep::Create => write!(f, "cannot create a temporary file to replace {path}"),
            WriteStep::Write => write!(f, "cannot write the temporary file that replaces {path}"),
            WriteStep::Sync => write!(
                f,
                "cannot sync the temporary file that replaces {path} to disk"
            ),
            WriteStep::Rename => write!(f, "cannot rename the temporary file over {path}"),
            WriteStep::SyncDirectory => write!(
                f,
                "replaced {path}, but cannot sync its directory, so after a power loss it may hold its old bytes"
            ),
        }
    }
}

impl Error for WriteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// An `io::Error` made from a [`WriteError`] has the kind of the operating
/// system's error and carries the `WriteError` as its inner error.
impl From<WriteError> for io::Error {
    fn from(error: WriteError) -> Self {
        Self::new(error.source.kind(), error)
    }
}

/// The number of temporary names that [`write_atomically`] tries before it
/// fails with `AlreadyExists`.
const NAMES: u32 = 16;

/// Replaces the file at `path` with `bytes`, so a reader, and after a
/// crash or a power loss the file itself, holds the old bytes or the new
/// bytes and never part of either.
///
/// It creates the new file `.<name>.<pid>.<n>.tmp` beside `path`, where
/// `<name>` is `path`'s file name, `<pid>` the process's id and `<n>` the
/// first of 0 to 15 at which nothing exists. A name that is taken, such as
/// one that a crash left behind or one that another thread of the process
/// is writing, is skipped, and the file or link there is never opened or
/// removed. It writes `bytes`, syncs the file to disk, renames it over
/// `path`, and on Unix then syncs `path`'s directory, `.` for a bare file
/// name, so the rename survives a power loss.
///
/// A symbolic link at `path` is replaced by a regular file and never
/// followed. The new file has the permission bits of `mode` on Unix, and
/// never the old file's. On Windows `mode` is ignored, the file inherits
/// its directory's access list, and the directory is not synced, because
/// NTFS journals the rename. The function creates no directory.
///
/// # Errors
///
/// Returns a [`WriteError`] that names the step that failed. A failure
/// before [`WriteStep::SyncDirectory`] leaves `path` unchanged and removes
/// the temporary file. `path` without a file name fails at
/// [`WriteStep::Create`] with `InvalidInput` before any file is opened,
/// and 16 taken names fail there with `AlreadyExists`. A directory sync
/// that fails with `EINVAL` or `ENOTSUP`, as on some SMB and FUSE mounts
/// that cannot sync a directory, counts as done.
pub fn write_atomically(path: &Path, bytes: &[u8], mode: Mode) -> Result<(), WriteError> {
    let fail = |step, source| WriteError {
        step,
        path: path.to_path_buf(),
        source,
    };
    let Some(name) = path.file_name() else {
        return Err(fail(
            WriteStep::Create,
            io::Error::new(io::ErrorKind::InvalidInput, "the path names no file"),
        ));
    };
    let (temporary, mut file) = create(path, name, mode).map_err(|e| fail(WriteStep::Create, e))?;
    let written = file
        .write_all(bytes)
        .map_err(|e| (WriteStep::Write, e))
        .and_then(|()| file.sync_all().map_err(|e| (WriteStep::Sync, e)));
    drop(file);
    #[expect(
        clippy::disallowed_methods,
        reason = "crumbkit-durable owns the rename of its temporary file over the target"
    )]
    let renamed =
        written.and_then(|()| fs::rename(&temporary, path).map_err(|e| (WriteStep::Rename, e)));
    if let Err((step, source)) = renamed {
        #[expect(
            clippy::disallowed_methods,
            reason = "crumbkit-durable removes its own temporary file after a failed step"
        )]
        let _ = fs::remove_file(&temporary);
        return Err(fail(step, source));
    }
    directory_sync_settles(platform::sync_directory(directory(path)))
        .map_err(|e| fail(WriteStep::SyncDirectory, e))
}

/// Creates the first temporary file for `path` whose name is free, and
/// returns its path and the open file.
fn create(path: &Path, name: &OsStr, mode: Mode) -> io::Result<(PathBuf, File)> {
    let mut n = 0;
    loop {
        let temporary = path.with_file_name(temporary_name(name, n));
        match platform::open_new(&temporary, mode) {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && n + 1 < NAMES => n += 1,
            Err(error) => return Err(error),
        }
    }
}

/// Returns the name of temporary file `n` for a target named `name`:
/// `.<name>.<pid>.<n>.tmp`.
pub(crate) fn temporary_name(name: &OsStr, n: u32) -> OsString {
    let mut temporary = OsString::from(".");
    temporary.push(name);
    temporary.push(format!(".{}.{n}.tmp", std::process::id()));
    temporary
}

/// Returns the directory that holds `path`, which is `.` for a bare file
/// name such as `commands.md`.
pub(crate) fn directory(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// `ENOTSUP` on Apple systems, where it differs from `EOPNOTSUPP` and std
/// gives it no error kind of its own.
#[cfg(target_vendor = "apple")]
const APPLE_ENOTSUP: Option<i32> = Some(45);
#[cfg(not(target_vendor = "apple"))]
const APPLE_ENOTSUP: Option<i32> = None;

/// Returns the result of a directory sync with `EINVAL` and `ENOTSUP`
/// counted as done, as SQLite counts them, because a file system that
/// cannot sync a directory returns them.
///
/// std gives `EINVAL` the kind `InvalidInput`, and `EOPNOTSUPP` and `ENOSYS`
/// the kind `Unsupported`. `ENOTSUP` is `EOPNOTSUPP` on Linux and the BSDs,
/// and a value of its own on Apple systems, which this checks by number.
pub(crate) fn directory_sync_settles(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::InvalidInput | io::ErrorKind::Unsupported
            ) || (APPLE_ENOTSUP.is_some() && error.raw_os_error() == APPLE_ENOTSUP) =>
        {
            Ok(())
        }
        result => result,
    }
}
