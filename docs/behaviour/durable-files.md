# Durable files

This page covers the crumbkit-durable crate, which replaces a file so that
a reader, a crash or a power loss never leaves it holding part of the old
bytes and part of the new, and locks a lock file so that two holders never
both hold it. The rustdoc of
[`write_atomically`](../../crates/crumbkit-durable/src/write.rs) and
[`PathLock`](../../crates/crumbkit-durable/src/lock.rs) describes the
items.

## The crumbkit-durable crate

The published crumbkit-durable crate depends on no other crumbkit crate,
so a tool that does not use crumbkit can replace and lock its files through
it. On Unix it depends on rustix's `fs` feature, for the `O_NOFOLLOW` and
`O_NONBLOCK` flags that std does not name, so rustix is a new dependency
on Unix for a tool outside the crumbkit workspace. It holds the file writes and locks that every tool needs to get
right in the same way, and prints nothing.

Why: ADR-0221, ADR-0222.

## Replacing a file

`write_atomically(path, bytes, mode)` replaces `path` in five steps:

1. It creates a new temporary file beside `path`, named
   `.<name>.<pid>.<n>.tmp`, where `<name>` is the target's file name,
   `<pid>` the process's id and `<n>` the first number from 0 to 15 at
   which nothing exists.
2. It writes `bytes` to the temporary file.
3. It syncs the temporary file to disk, which is `F_FULLFSYNC` on macOS.
4. It renames the temporary file over `path`.
5. On Unix it syncs `path`'s directory, so the rename itself survives a
   power loss. A bare file name, such as `commands.md`, means the current
   directory.

The open in step 1 uses `create_new`, which never follows a symbolic link
and never opens a file that already exists. A name that is taken, such as
one that a crash left behind or one that another thread of the process is
writing, is skipped, and the file there is never opened, truncated or
removed. When all 16 names are taken, the write fails at the create step
with `AlreadyExists` and writes nothing. The 16 names count the temporary
files that one process can hold for one target at once.

A symbolic link at `path` is replaced by a regular file, and the file it
leads to is left unchanged. The new file gets the permission bits of
`mode`: `Mode::Private` is `0600`, for a file such as a key, and
`Mode::Umask` is `0666` less the process's umask. It never keeps the old
file's bits. The function creates no directory.

Why: ADR-0221.

## Failures

A failure returns a `WriteError` with the step that failed, the target's
path and the operating system's error. Its `Display` names the step and the
target, and converting it to an `io::Error` keeps the error's kind.

| Step | What fails | State left |
| --- | --- | --- |
| `Create` | the path has no file name (`InvalidInput`), the directory is missing or read-only, the file name is longer than about 236 bytes, so the temporary name, up to 19 bytes longer, passes a 255-byte name limit (`ENAMETOOLONG`), or 16 names are taken (`AlreadyExists`) | nothing written |
| `Write` | no space, an I/O error | target unchanged, temporary file removed |
| `Sync` | the file system cannot flush the file | target unchanged, temporary file removed |
| `Rename` | the target is a directory, the rename crosses devices, or Windows refuses to share the target | target unchanged, temporary file removed |
| `SyncDirectory` | the directory cannot be opened or synced | target holds the new bytes; after a power loss it may hold the old bytes, never part of either |

A directory sync that fails with `EINVAL` or `ENOTSUP` counts as done, as
SQLite counts it, because some file systems, such as SMB and some FUSE
mounts, cannot sync a directory. `ENOSYS` counts as done too, because std
gives it the kind of `EOPNOTSUPP`, `Unsupported`.

Why: ADR-0221.

## The temporary file's lifecycle

| Transition | Step | A crash here leaves | Recovery |
| --- | --- | --- | --- |
| absent to created | `open` with `O_CREAT \| O_EXCL` | an empty temporary file, target unchanged | the next write skips the taken name |
| created to written | `write_all` | a partial temporary file, target unchanged | as above |
| written to synced | `sync_all` | a full temporary file, target unchanged | as above |
| synced to renamed, when it becomes the target | `rename` | the target with the new bytes; after a power loss before the directory sync, the old or the new bytes | none needed |
| any state to removed | a step after the create fails | a temporary file, when the removal itself fails | the next write skips the taken name |

A target is never part old and part new, because its bytes are synced
before the rename that publishes them. Nothing removes a temporary file
that a crash left behind.

Why: ADR-0221.

## Locking a lock file

`PathLock::acquire_unless(path, deadline, give_up)` takes an exclusive lock
on `path`, a lock file that the caller names for the lock alone, and
`PathLock::acquire(path, deadline)` is the same call with a `give_up` that
returns false. The lock is released when the `PathLock` drops or the
process ends. crumbkit never renames, truncates or removes a lock file, so
the file stays, empty and unlocked, after the release.

1. It opens `path` for writing, creating it with `0600` on Unix when it is
   absent. The open passes `O_NOFOLLOW`, so a symbolic link at `path` is
   refused, and `O_NONBLOCK`, so a FIFO at `path` that no program reads
   is refused at once instead of blocking the open past the deadline. On
   Unix it then compares the file's owner, from `fstat`, with the caller's
   effective uid. When they differ it closes the file and returns `Io` at
   `Open` with the kind `PermissionDenied` and a message naming the path
   and the owner's uid, so a user who created the file first in a shared
   directory cannot make the caller lock it. Windows does not check the
   owner.
2. It calls `try_lock`, which is `flock` on Unix and `LockFileEx` on
   Windows. While another holder has the lock, it calls `give_up`, returns
   `Busy` when that returns true or the deadline has passed, and otherwise
   sleeps 5 ms, or what is left before the deadline, and tries the same
   file again. A deadline that has passed tries once.
3. On Unix, once it holds the lock, it compares the device and inode of the
   open file, from `fstat`, with those of the file at `path`, from `lstat`.
   When they differ or `path` is gone, another program replaced or removed
   the file while the call waited, so it releases the lock and starts again
   at step 1. That is not contention, so the deadline does not stop it, and
   after the third reopen a fourth change returns `Busy`.

While no other program replaces or removes the lock file, two holders
therefore never both return from `acquire_unless` holding different files
that were at `path`. Another program can still remove or
replace the lock file while a holder has it, and a second acquirer then
locks the new file. `still_current` compares the identities again and is
false once that happened, so a holder about to change what the lock guards
calls it first. `waited` returns how long the acquire took, from its start
to the moment it held the lock, including the open and each reopen, so a
tool that records its waiting on locks adds it.

The lock belongs to the open file, so it is not re-entrant: a second
acquire of one path in one process waits for the first. A child that
`fork` makes without `exec` keeps the lock until it exits, and std's
`O_CLOEXEC` closes the file in every program that `exec` starts.

An uncontended acquire makes four system calls, the open, `flock`, `fstat`
and `lstat`, and a wait repeats only `flock`.

Why: ADR-0222, ADR-0231.

## Lock failures

A failure returns a `LockError`. `Busy` names the path and how long the
call waited, and converts to an `io::Error` of kind `TimedOut`. `Io` names
the path, the `LockStep` that failed and the operating system's error, and
converts to an `io::Error` of that error's kind.

| Error | What fails | State left |
| --- | --- | --- |
| `Io` at `Open` | the directory is missing, permission is refused, the file on Unix belongs to another user (`PermissionDenied`), a symbolic link is at the path (`ELOOP`, or `EMLINK` on FreeBSD), or a FIFO that no program reads is at the path (`ENXIO`) | nothing held |
| `Io` at `Lock` | an error other than the lock being held | nothing held |
| `Busy` | another holder kept the lock past the deadline, `give_up` returned true, or the file changed a fourth time | nothing held |
| `Io` at `Identify` | `fstat` fails, or `lstat` fails other than with not found | the lock released |

Why: ADR-0222.

## The lock's lifecycle

| Transition | Event | A crash here leaves | Recovery |
| --- | --- | --- | --- |
| absent to present | the open with `O_CREAT \| O_NOFOLLOW` and `0600` | an empty lock file, nothing held | none needed, because nothing removes it |
| present to waiting | `try_lock` finds the lock held | nothing held | none |
| waiting to busy | the deadline passes or `give_up` returns true | nothing held | the caller tries later |
| waiting to locked | `try_lock` succeeds | a lock that the operating system releases on exit | none |
| locked to reopened | the identities differ or the path is gone | the lock on the old file released | the call opens the path again, at most 3 times |
| locked to held | the identities match | a lock that the operating system releases on exit | none |
| held to stale | another program removes or replaces the file, on Unix | a second acquirer may hold the new file | the holder calls `still_current` before its next destructive step |
| held to released | the `PathLock` drops or the process ends | the lock file, unlocked | none |

`acquire_unless` returns only after the locked file was the file at the
path, checked after the lock was taken, so a caller's first read under the
lock comes after the check.

Why: ADR-0222.

## Platforms

- Linux is tested on every pull request, and macOS on each push to `main`.
- On Windows, tested each night, the directory is not synced, because std
  cannot open a directory there without `FILE_FLAG_BACKUP_SEMANTICS`, and
  NTFS journals the rename. `mode` is ignored, and the file inherits its
  directory's access list.
- On Windows a lock file opens without `FILE_SHARE_DELETE`, so no process
  can remove it or rename another file over it while it is open. The
  identity check does not run, `still_current` returns true, and a
  symbolic link at the path is followed. The lock is mandatory there, so a
  read or write of the lock file fails while another handle holds it.
- Other Unix systems build and are untested. Network file systems are out
  of scope.

Why: ADR-0221, ADR-0222, ADR-0003.
