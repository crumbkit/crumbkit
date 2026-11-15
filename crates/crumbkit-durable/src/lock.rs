//! Locking a lock file, and checking that it is still the file at its path.

use std::error::Error;
use std::fmt;
use std::fs::{File, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::platform::{self, Identity};

#[cfg(test)]
mod tests;

/// How long [`PathLock::acquire_unless`] sleeps between two tries of a
/// held lock.
const POLL: Duration = Duration::from_millis(5);

/// The number of times [`PathLock::acquire_unless`] opens the path again
/// after the file it locked was no longer the file at the path.
const REOPENS: u32 = 3;

/// An exclusive lock on a lock file, released when it drops or the process
/// ends.
///
/// The lock file is a file the caller names for the lock alone. crumbkit
/// never renames, truncates or removes it, and leaves it in place after the
/// lock is released. While no other program replaces or removes the file,
/// two holders never both return from [`PathLock::acquire_unless`] holding
/// different files that were each at the path. Another program can still
/// remove or replace the file while a holder has it on Unix, after which a
/// second acquire locks the new file; the holder learns of this from
/// [`PathLock::still_current`].
///
/// The lock is a `flock` on Unix and `LockFileEx` on Windows, and belongs
/// to the open file, so a second acquire of one path in one process waits
/// for the first. A child that `fork` makes without `exec` keeps the lock
/// until it exits, and the open's `O_CLOEXEC` closes the file in every
/// program that `exec` starts.
#[must_use = "the lock is released when it drops"]
#[derive(Debug)]
pub struct PathLock {
    /// The open lock file. Closing it when it drops releases the lock.
    #[expect(
        dead_code,
        reason = "the lock lives as long as the file stays open, and nothing reads it"
    )]
    file: File,
    path: PathBuf,
    identity: Identity,
}

/// The step of [`PathLock::acquire_unless`] that failed.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LockStep {
    /// Opening or creating the lock file. Nothing is held.
    Open,
    /// Taking the lock. Nothing is held.
    Lock,
    /// Reading the identity of the locked file or of the file at the path.
    /// The lock was released.
    Identify,
}

/// Why [`PathLock::acquire_unless`] returned without a lock.
#[non_exhaustive]
#[derive(Debug)]
pub enum LockError {
    /// Another holder kept the lock until the deadline passed or `give_up`
    /// returned true, or the file at the path changed four times while it
    /// was being locked.
    Busy {
        /// The lock file.
        path: PathBuf,
        /// How long the call waited.
        waited: Duration,
    },
    /// The operating system refused a step.
    Io {
        /// The lock file.
        path: PathBuf,
        /// The step that failed.
        step: LockStep,
        /// The operating system's error.
        source: io::Error,
    },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Busy { path, waited } => write!(
                f,
                "cannot lock {} within {} ms, because another holder has it",
                path.display(),
                waited.as_millis()
            ),
            Self::Io { path, step, .. } => {
                let path = path.display();
                match step {
                    LockStep::Open => write!(f, "cannot open the lock file {path}"),
                    LockStep::Lock => write!(f, "cannot lock {path}"),
                    LockStep::Identify => {
                        write!(f, "cannot check that the locked file is still {path}")
                    }
                }
            }
        }
    }
}

impl Error for LockError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Busy { .. } => None,
            Self::Io { source, .. } => Some(source),
        }
    }
}

/// An `io::Error` made from a [`LockError`] has the kind `TimedOut` for
/// [`LockError::Busy`] and the kind of the operating system's error for
/// [`LockError::Io`], and carries the `LockError` as its inner error.
impl From<LockError> for io::Error {
    fn from(error: LockError) -> Self {
        let kind = match &error {
            LockError::Busy { .. } => io::ErrorKind::TimedOut,
            LockError::Io { source, .. } => source.kind(),
        };
        Self::new(kind, error)
    }
}

impl PathLock {
    /// Locks the lock file at `path`, creating it when it is absent, and
    /// waits until `deadline` while another holder has it.
    ///
    /// It is [`PathLock::acquire_unless`] with a `give_up` that returns
    /// false.
    ///
    /// # Errors
    ///
    /// As [`PathLock::acquire_unless`].
    pub fn acquire(path: &Path, deadline: Instant) -> Result<Self, LockError> {
        Self::acquire_unless(path, deadline, &|| false)
    }

    /// Locks the lock file at `path`, creating it with `0600` on Unix when
    /// it is absent, and waits while another holder has it until `deadline`
    /// passes or `give_up` returns true.
    ///
    /// While the lock is held it tries again every 5 ms, and calls
    /// `give_up` before each sleep. A `deadline` that has passed tries
    /// once. Once it holds the lock it checks, on Unix, that the file it
    /// locked is still the file at `path`, by device and inode. When
    /// another program replaced or removed the file meanwhile, it releases
    /// the lock and opens `path` again, whatever the deadline, at most 3
    /// times.
    ///
    /// A lock is not re-entrant: a second acquire of one path in one
    /// process waits for the first.
    ///
    /// # Errors
    ///
    /// Returns [`LockError::Busy`] when the deadline passes or `give_up`
    /// returns true while another holder has the lock, and after the third
    /// reopen finds the file changed again. Returns [`LockError::Io`] when
    /// the open fails, which includes a symbolic link at `path` on Unix and
    /// a FIFO there that no program reads,
    /// when the lock fails other than by being held, or when reading the
    /// identity of either file fails other than by the path being gone.
    pub fn acquire_unless(
        path: &Path,
        deadline: Instant,
        give_up: &dyn Fn() -> bool,
    ) -> Result<Self, LockError> {
        let started = Instant::now();
        let file = open(path)?;
        lock_opened(path, file, started, deadline, give_up)
    }

    /// Returns whether the locked file is still the file at its path, which
    /// is false once another program replaced or removed it. A caller that
    /// is about to change what the lock guards checks this first.
    ///
    /// On Windows it returns true, because no program can replace or remove
    /// the lock file while it is open.
    ///
    /// # Errors
    ///
    /// Returns the operating system's error when it cannot read the path's
    /// metadata for a reason other than the path being gone.
    pub fn still_current(&self) -> io::Result<bool> {
        Ok(platform::path_identity(&self.path)? == Some(self.identity))
    }
}

/// Opens the lock file at `path`, creating it when it is absent.
fn open(path: &Path) -> Result<File, LockError> {
    platform::open_lock(path).map_err(|source| LockError::Io {
        path: path.to_path_buf(),
        step: LockStep::Open,
        source,
    })
}

/// Locks `file`, which was opened at `path` after `started`, waits as
/// [`PathLock::acquire_unless`] says, and opens `path` again while the
/// locked file is no longer the file there. `Busy` counts its wait from
/// `started`, so the wait includes the open.
fn lock_opened(
    path: &Path,
    mut file: File,
    started: Instant,
    deadline: Instant,
    give_up: &dyn Fn() -> bool,
) -> Result<PathLock, LockError> {
    let fail = |step, source| LockError::Io {
        path: path.to_path_buf(),
        step,
        source,
    };
    let busy = || LockError::Busy {
        path: path.to_path_buf(),
        waited: started.elapsed(),
    };
    let mut reopens = 0;
    loop {
        #[expect(
            clippy::disallowed_methods,
            reason = "crumbkit-durable owns taking the lock on a lock file"
        )]
        let tried = file.try_lock();
        match tried {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                let now = Instant::now();
                if give_up() || now >= deadline {
                    return Err(busy());
                }
                thread::sleep(POLL.min(deadline - now));
                continue;
            }
            Err(TryLockError::Error(source)) => return Err(fail(LockStep::Lock, source)),
        }
        // The lock is held from here, and dropping `file` releases it.
        let identity = platform::file_identity(&file).map_err(|e| fail(LockStep::Identify, e))?;
        let at_path = platform::path_identity(path).map_err(|e| fail(LockStep::Identify, e))?;
        if at_path == Some(identity) {
            return Ok(PathLock {
                file,
                path: path.to_path_buf(),
                identity,
            });
        }
        // Another program replaced or removed the file while this call
        // waited, so the lock is on a file that `path` no longer reaches.
        // This is not contention, so the deadline does not stop it.
        if reopens == REOPENS {
            return Err(busy());
        }
        reopens += 1;
        drop(file);
        file = open(path)?;
    }
}
