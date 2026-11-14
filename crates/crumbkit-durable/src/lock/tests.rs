use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use super::{LockError, PathLock};

/// A deadline far enough away that no test reaches it.
fn later() -> Instant {
    Instant::now() + Duration::from_secs(60)
}

/// Starts a thread that acquires `path` with a deadline far away, and
/// returns once the thread is waiting on the lock, with the thread's handle.
///
/// The thread's `give_up` signals on its first call, which comes after its
/// first `try_lock` found the lock held, so the caller knows the thread
/// holds a descriptor of the file at `path` as it was then.
fn wait_in_thread(path: &Path) -> thread::JoinHandle<Result<PathLock, LockError>> {
    let (waiting, waited) = mpsc::channel();
    let path = path.to_path_buf();
    let handle = thread::spawn(move || {
        let signal = std::sync::Mutex::new(Some(waiting));
        PathLock::acquire_unless(&path, later(), &|| {
            if let Some(waiting) = signal.lock().unwrap().take() {
                waiting.send(()).unwrap();
            }
            false
        })
    });
    waited.recv().unwrap();
    handle
}

/// Creates an empty file in `directory` and renames it over `path`, as
/// another program that replaces a lock file would.
#[cfg(unix)]
fn replace(directory: &Path, path: &Path) {
    let new = directory.join("new");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test replaces the lock file as another program would"
    )]
    {
        fs::write(&new, "").unwrap();
        fs::rename(&new, path).unwrap();
    }
}

/// Removes the file at `path`, as another program that cleans up a lock
/// file would.
#[cfg(unix)]
fn remove(path: &Path) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the test removes the lock file as another program would"
    )]
    fs::remove_file(path).unwrap();
}

#[test]
fn a_second_acquire_waits_until_the_first_lock_drops() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");
    let first = PathLock::acquire(&path, later()).unwrap();

    let second = wait_in_thread(&path);
    drop(first);

    let second = second.join().unwrap().unwrap();
    assert!(second.still_current().unwrap());
}

#[test]
fn an_acquire_past_its_deadline_returns_busy_and_names_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");
    let _held = PathLock::acquire(&path, later()).unwrap();

    let error = PathLock::acquire(&path, Instant::now() + Duration::from_millis(30)).unwrap_err();

    let LockError::Busy {
        path: named,
        waited,
    } = &error
    else {
        panic!("expected Busy, got {error:?}");
    };
    assert_eq!(named, &path);
    assert!(*waited >= Duration::from_millis(30), "waited {waited:?}");
    assert!(error.to_string().contains(&path.display().to_string()));
    assert_eq!(io_kind(error), std::io::ErrorKind::TimedOut);
}

fn io_kind(error: LockError) -> std::io::ErrorKind {
    std::io::Error::from(error).kind()
}

#[test]
fn an_acquire_with_a_passed_deadline_tries_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");

    let free = PathLock::acquire(&path, Instant::now()).unwrap();

    let started = Instant::now();
    let error = PathLock::acquire(&path, Instant::now()).unwrap_err();
    assert!(matches!(error, LockError::Busy { .. }), "{error:?}");
    assert!(started.elapsed() < Duration::from_secs(1));
    drop(free);

    // A file opened before another program replaced it is locked, found to
    // be the old file and opened again, whatever the deadline.
    #[cfg(unix)]
    {
        let old = super::open(&path).unwrap();
        replace(dir.path(), &path);
        let lock = super::lock_opened(&path, old, Instant::now(), &|| false).unwrap();
        assert!(lock.still_current().unwrap());
    }
}

#[test]
fn give_up_ends_the_wait_with_busy_before_the_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");
    let _held = PathLock::acquire(&path, later()).unwrap();

    let error = PathLock::acquire_unless(&path, later(), &|| true).unwrap_err();

    let LockError::Busy { waited, .. } = error else {
        panic!("expected Busy, got {error:?}");
    };
    assert!(waited < Duration::from_secs(1), "waited {waited:?}");
}

#[cfg(unix)]
#[test]
fn a_holder_waiting_when_the_lock_file_is_replaced_locks_the_file_now_at_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");
    let first = PathLock::acquire(&path, later()).unwrap();

    let second = wait_in_thread(&path);
    replace(dir.path(), &path);
    drop(first);

    let second = second.join().unwrap().unwrap();
    assert!(second.still_current().unwrap());
    let third = PathLock::acquire(&path, Instant::now()).unwrap_err();
    assert!(matches!(third, LockError::Busy { .. }), "{third:?}");
}

#[cfg(unix)]
#[test]
fn a_holder_waiting_when_the_lock_file_is_removed_locks_a_new_file_at_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");
    let first = PathLock::acquire(&path, later()).unwrap();

    let second = wait_in_thread(&path);
    remove(&path);
    drop(first);

    let second = second.join().unwrap().unwrap();
    assert!(second.still_current().unwrap());
    let third = PathLock::acquire(&path, Instant::now()).unwrap_err();
    assert!(matches!(third, LockError::Busy { .. }), "{third:?}");
}

#[cfg(unix)]
#[test]
fn still_current_is_true_while_the_file_stays_and_false_once_it_is_replaced_or_removed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");

    let replaced = PathLock::acquire(&path, later()).unwrap();
    assert!(replaced.still_current().unwrap());
    replace(dir.path(), &path);
    assert!(!replaced.still_current().unwrap());
    drop(replaced);

    let removed = PathLock::acquire(&path, later()).unwrap();
    assert!(removed.still_current().unwrap());
    remove(&path);
    assert!(!removed.still_current().unwrap());
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_at_the_lock_path_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    let path = dir.path().join("state.lock");
    std::os::unix::fs::symlink(&target, &path).unwrap();

    let error = PathLock::acquire(&path, later()).unwrap_err();

    assert!(
        matches!(
            error,
            LockError::Io {
                step: super::LockStep::Open,
                ..
            }
        ),
        "{error:?}"
    );
    assert!(!target.exists(), "the open followed the link");
}

#[cfg(windows)]
#[test]
fn a_held_lock_file_cannot_be_removed_or_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");
    let new = dir.path().join("new");
    let held = PathLock::acquire(&path, later()).unwrap();

    #[expect(
        clippy::disallowed_methods,
        reason = "the test tries to remove and replace the lock file as another program would"
    )]
    {
        fs::write(&new, "").unwrap();
        assert!(fs::remove_file(&path).is_err());
        assert!(fs::rename(&new, &path).is_err());
    }
    assert!(held.still_current().unwrap());
}

#[test]
fn dropping_the_lock_releases_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.lock");

    drop(PathLock::acquire(&path, later()).unwrap());

    drop(PathLock::acquire(&path, Instant::now()).unwrap());

    // Windows refuses a read of a locked file, so the read comes after the
    // release.
    assert_eq!(fs::read(&path).unwrap(), b"");
}
