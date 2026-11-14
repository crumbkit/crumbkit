# Durable files

This page covers the crumbkit-durable crate, which replaces a file so that
a reader, a crash or a power loss never leaves it holding part of the old
bytes and part of the new. The rustdoc of
[`write_atomically`](../../crates/crumbkit-durable/src/write.rs) describes
the item.

## The crumbkit-durable crate

The published crumbkit-durable crate depends only on the standard library
and on no other crumbkit crate, so a tool that does not use crumbkit can
replace its files through it. It holds the file writes that every tool
needs to get right in the same way, and prints nothing.

Why: ADR-0221.

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

## Platforms

- Linux is tested on every pull request, and macOS on each push to `main`.
- On Windows, tested each night, the directory is not synced, because std
  cannot open a directory there without `FILE_FLAG_BACKUP_SEMANTICS`, and
  NTFS journals the rename. `mode` is ignored, and the file inherits its
  directory's access list.
- Other Unix systems build and are untested. Network file systems are out
  of scope.

Why: ADR-0221, ADR-0003.
