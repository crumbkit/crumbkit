use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use super::{Mode, WriteStep, directory, write_atomically};

/// Returns the names in `directory`, sorted.
fn names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// Returns the path of temporary name `n` for `target`, spelled out here
/// so the tests also check the name that the docs promise.
fn temporary(target: &Path, n: u32) -> PathBuf {
    let name = target.file_name().unwrap().to_str().unwrap();
    target.with_file_name(format!(".{name}.{}.{n}.tmp", std::process::id()))
}

/// Writes `text` to `path` in place, as a file that a test sets up.
fn put(path: &Path, text: impl AsRef<[u8]>) {
    #[expect(
        clippy::disallowed_methods,
        reason = "the tests write the files that write_atomically then finds"
    )]
    fs::write(path, text).unwrap();
}

#[test]
fn write_atomically_replaces_the_file_and_leaves_no_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("notes.txt");
    put(&target, "old");

    write_atomically(&target, b"new", Mode::Umask).unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert_eq!(names(dir.path()), ["notes.txt"]);
}

#[test]
fn a_reader_sees_the_old_bytes_or_the_new_bytes_while_another_thread_replaces_the_file() {
    const SIZE: usize = 64 * 1024;
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("state");
    write_atomically(&target, &vec![0; SIZE], Mode::Umask).unwrap();
    let done = AtomicBool::new(false);

    thread::scope(|scope| {
        scope.spawn(|| {
            for round in 1..=100_u8 {
                write_atomically(&target, &vec![round; SIZE], Mode::Umask).unwrap();
            }
            done.store(true, Ordering::SeqCst);
        });
        while !done.load(Ordering::SeqCst) {
            let bytes = fs::read(&target).unwrap();
            assert_eq!(bytes.len(), SIZE);
            assert!(
                bytes.iter().all(|byte| *byte == bytes[0]),
                "a read mixed two versions"
            );
        }
    });

    assert_eq!(fs::read(&target).unwrap(), vec![100; SIZE]);
}

#[test]
fn a_taken_temporary_name_is_skipped_and_the_file_there_is_left_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("notes.txt");
    let taken = temporary(&target, 0);
    put(&taken, "left by a crash");

    write_atomically(&target, b"new", Mode::Umask).unwrap();

    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert_eq!(fs::read(&taken).unwrap(), b"left by a crash");
    assert!(!temporary(&target, 1).exists());
}

#[test]
fn two_threads_that_write_one_target_at_once_both_succeed_and_the_target_holds_one_of_their_texts()
{
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("notes.txt");

    thread::scope(|scope| {
        let first = scope.spawn(|| write_atomically(&target, b"first", Mode::Umask));
        let second = scope.spawn(|| write_atomically(&target, b"second", Mode::Umask));
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });

    let text = fs::read(&target).unwrap();
    assert!(text == b"first" || text == b"second");
    assert_eq!(names(dir.path()), ["notes.txt"]);
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_at_the_temporary_name_is_never_followed() {
    let dir = tempfile::tempdir().unwrap();
    let victim = dir.path().join("victim");
    put(&victim, "victim");
    let target = dir.path().join("notes.txt");
    let link = temporary(&target, 0);
    std::os::unix::fs::symlink(&victim, &link).unwrap();

    write_atomically(&target, b"new", Mode::Umask).unwrap();

    assert_eq!(fs::read(&victim).unwrap(), b"victim");
    assert_eq!(fs::read_link(&link).unwrap(), victim);
    assert_eq!(fs::read(&target).unwrap(), b"new");
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_at_the_target_is_replaced_and_the_file_it_leads_to_is_left_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let victim = dir.path().join("victim");
    put(&victim, "victim");
    let target = dir.path().join("notes.txt");
    std::os::unix::fs::symlink(&victim, &target).unwrap();

    write_atomically(&target, b"new", Mode::Umask).unwrap();

    assert!(fs::symlink_metadata(&target).unwrap().file_type().is_file());
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert_eq!(fs::read(&victim).unwrap(), b"victim");
}

#[cfg(unix)]
#[test]
fn a_private_file_has_mode_0600_and_a_umask_file_has_the_mode_the_umask_allows() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().unwrap();
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    // std creates a file with 0666 less the umask, so this file shows what
    // the umask allows without reading the umask, which only a call that
    // also changes it can do.
    let reference = dir.path().join("reference");
    put(&reference, "");
    let private = dir.path().join("private");
    let shared = dir.path().join("shared");

    write_atomically(&private, b"key", Mode::Private).unwrap();
    write_atomically(&shared, b"text", Mode::Umask).unwrap();

    assert_eq!(mode(&private), 0o600);
    assert_eq!(mode(&shared), mode(&reference));
}

#[test]
fn a_failed_rename_leaves_the_target_unchanged_and_removes_the_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("notes");
    fs::create_dir(&target).unwrap();
    put(&target.join("inside"), "kept");

    let error = write_atomically(&target, b"new", Mode::Umask).unwrap_err();

    assert_eq!(error.step, WriteStep::Rename);
    assert_eq!(error.path, target);
    assert_eq!(fs::read(target.join("inside")).unwrap(), b"kept");
    assert_eq!(names(dir.path()), ["notes"]);
}

#[test]
fn a_path_without_a_file_name_is_refused_before_anything_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let inner = dir.path().join("inner");
    fs::create_dir(&inner).unwrap();
    let target = inner.join("..");

    let error = write_atomically(&target, b"new", Mode::Umask).unwrap_err();

    assert_eq!(error.step, WriteStep::Create);
    assert_eq!(error.source.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(names(dir.path()), ["inner"]);
    assert_eq!(names(&inner), Vec::<String>::new());
}

#[test]
fn a_missing_directory_fails_at_create_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("missing").join("notes.txt");

    let error = write_atomically(&target, b"new", Mode::Umask).unwrap_err();

    assert_eq!(error.step, WriteStep::Create);
    assert_eq!(error.source.kind(), io::ErrorKind::NotFound);
    assert_eq!(names(dir.path()), Vec::<String>::new());
}

#[test]
fn the_directory_of_a_bare_file_name_is_the_current_directory() {
    assert_eq!(directory(Path::new("commands.md")), Path::new("."));
    assert_eq!(directory(Path::new("docs/commands.md")), Path::new("docs"));
    assert_eq!(directory(Path::new("/commands.md")), Path::new("/"));
}

#[test]
fn sixteen_taken_names_fail_with_already_exists_and_write_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("notes.txt");
    for n in 0..16 {
        put(&temporary(&target, n), format!("left {n}"));
    }

    let error = write_atomically(&target, b"new", Mode::Umask).unwrap_err();

    assert_eq!(error.step, WriteStep::Create);
    assert_eq!(error.source.kind(), io::ErrorKind::AlreadyExists);
    assert!(!target.exists());
    assert!(!temporary(&target, 16).exists());
    for n in 0..16 {
        assert_eq!(
            fs::read(temporary(&target, n)).unwrap(),
            format!("left {n}").as_bytes()
        );
    }
}

// The errno values are the ones Linux and macOS give EINVAL, ENOTSUP and
// EIO. No file system in CI returns the first two from a directory sync,
// so the rule is tested on the errors themselves.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn a_directory_sync_that_fails_with_einval_or_enotsup_counts_as_done_and_one_with_eio_fails() {
    const EINVAL: i32 = 22;
    #[cfg(target_os = "linux")]
    const ENOTSUP: i32 = 95;
    #[cfg(target_os = "macos")]
    const ENOTSUP: i32 = 45;
    const EIO: i32 = 5;
    let sync = |errno| super::directory_sync_settles(Err(io::Error::from_raw_os_error(errno)));

    assert!(sync(EINVAL).is_ok());
    assert!(sync(ENOTSUP).is_ok());
    assert_eq!(sync(EIO).unwrap_err().raw_os_error(), Some(EIO));
}
