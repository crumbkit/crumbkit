# crumbkit-durable

crumbkit-durable replaces a file atomically, and locks a lock file. A
reader of a replaced file sees the old bytes or the new bytes, and after a
crash or a power loss the file holds one or the other, never part of
either. A lock is held by one holder at a time, on the file that is at the
path. The crate depends on no other crumbkit crate.

## Adding it

```toml
[dependencies]
crumbkit-durable = "0.5"
```

## Example

[`write_atomically`] takes the target, the bytes and a [`Mode`], and
[`PathLock::acquire`] takes the lock file and a deadline:

```rust
use std::time::{Duration, Instant};
use crumbkit_durable::{Mode, PathLock, write_atomically};

let dir = std::env::temp_dir().join(format!("crumbkit-durable-{}", std::process::id()));
std::fs::create_dir_all(&dir).unwrap();
let config = dir.join("config.toml");

let lock = PathLock::acquire(&dir.join("config.lock"), Instant::now() + Duration::from_secs(5)).unwrap();
write_atomically(&config, b"theme = \"dark\"\n", Mode::Umask).unwrap();
drop(lock);
assert_eq!(std::fs::read(&config).unwrap(), b"theme = \"dark\"\n");
std::fs::remove_dir_all(&dir).unwrap();
```

`Mode::Private` gives the file `0600` on Unix, and `Mode::Umask` gives it `0666` less the process's umask, as
`File::create` does.

## What it promises

[`write_atomically`] writes the bytes to a new temporary file beside the
target, syncs that file to disk, renames it over the target, and on Unix
syncs the target's directory, so the rename survives a power loss. The
temporary file is `.<name>.<pid>.<n>.tmp`. A name that is already taken,
such as one that a crash left behind, is skipped and its file left alone,
so two threads of one process can replace one file at once.

A symbolic link at the target is replaced by a regular file and never
followed. The new file gets the permission bits of its `Mode`, never the
old file's. The function creates no directory.

A failure returns a [`WriteError`] whose `step` names the step that failed.
A failure before the directory sync leaves the target unchanged and removes
the temporary file. A failed directory sync means the target already holds
the new bytes, and after a power loss it may hold the old ones. A directory
sync that fails with `EINVAL`, `ENOTSUP` or `ENOSYS` counts as done, because
some SMB and FUSE mounts cannot sync a directory.

[`PathLock::acquire`] creates the lock file with `0600` when it is absent,
refuses a symbolic link and a file that another user owns on Unix, and
waits until the deadline while another holder has the lock. Once it holds
the lock, it checks on Unix that the file it locked is still the file at
the path, and opens the path again when another program replaced or removed
it. While no other program replaces or removes the file, two holders never
both hold different files. The crate never removes a lock file, and
[`PathLock::still_current`] tells a holder that another program did. A
failure returns a [`LockError`], which is `Busy` when the wait ends. A
taken lock reports how long its acquire took through [`PathLock::waited`].

The owner check applies to root too. Every acquire fails on a mount that
reports one owner for every file, such as NFS with `all_squash`, or CIFS or
FUSE with `uid=`, and for root on NFS with `root_squash`. Keep lock files on
a local file system.

## Platforms

Linux is tested on every pull request, macOS on each push to `main`, and
Windows each night.
On Windows the `Mode` is ignored, so the file inherits its directory's
access list, and the directory is not synced, because NTFS journals the
rename. A lock file there cannot be removed or replaced while it is open,
so the identity check does not run. Other Unix systems build but are
untested, and network file systems are out of scope.

The [durable files docs] describe the lifecycles of the temporary file and
the lock, and [ADR-0221] and [ADR-0222] record why.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[`write_atomically`]: src/write.rs
[`Mode`]: src/write.rs
[`WriteError`]: src/write.rs
[`PathLock::acquire`]: src/lock.rs
[`PathLock::still_current`]: src/lock.rs
[`PathLock::waited`]: src/lock.rs
[`LockError`]: src/lock.rs
[ADR-0221]: ../../docs/decisions/api/effects/0221-the-durable-crate.md
[ADR-0222]: ../../docs/decisions/api/effects/0222-a-path-lock-checks-its-file-is-still-at-the-path.md
[durable files docs]: ../../docs/behaviour/durable-files.md
