# ADR-0221. The durable crate

- Date: 2026-11-14
- Status: accepted

## Context

Several of crumbkit's crates replace a file through a temporary file and a
rename, each with its own code, and the copies differ. crumbkit's
`files::write_atomically` syncs the file and never its directory, so after a power loss the rename
may be lost. It names its temporary file by the process's id alone, so a
file that a crash left at that name makes every later write by a process
with the same id fail, and two threads of one process cannot replace one
file at once. cargo-crumbkit's `docs` and `tool-skill draft` write in
place with `fs::write`, so a crash leaves a cut-off file. crumbkit-usage
creates its temporary files with `truncate` in place of `create_new`, so it
fills whatever is at that name.

## Decision

- A new published crate, crumbkit-durable, holds the file writes whose
  correctness every tool needs in the same form. It depends only on the
  standard library and on no other crumbkit crate, so a tool that does not
  use crumbkit can adopt it, and it shares the workspace version
  (ADR-0012).
- Its first item is `write_atomically(path, bytes, mode)`. It creates
  `.<name>.<pid>.<n>.tmp` beside `path` with `create_new`, where `<n>` is
  the first of 0 to 15 that is free, writes the bytes, syncs the file,
  renames it over `path`, and on Unix syncs `path`'s directory, `.` for a
  bare file name. A taken name is skipped and its file left alone, and 16
  taken names fail with `AlreadyExists`.
- `Mode::Private` creates the file with `0600` and `Mode::Umask` with
  `0666` less the umask. The new file never keeps the old file's bits, and
  a symbolic link at `path` is replaced, never followed.
- `WriteError` carries the failed `WriteStep`, the target's path and the
  `io::Error`, and converts into an `io::Error` of the same kind, so a
  caller that returns `io::Result` adopts the function with `?`.
- A directory sync that fails with `EINVAL` or `ENOTSUP` counts as done, as
  SQLite counts it, because some SMB and FUSE mounts cannot sync a
  directory. Any other error returns `SyncDirectory` after the target was
  replaced.
- On Windows the directory is not synced, because std cannot open a
  directory there and NTFS journals the rename, and `mode` is ignored. Each
  shortcut carries a RUST-10 comment.
- No caller moves in this change. The callers move onto the crate, and
  crumbkit's `files::write_atomically` goes, in a later change.

## Consequences

A write now pays a directory open and sync after the rename, which on
macOS is a second full flush of several milliseconds. The callers that will
use it write rarely: an install, a usage-log trim, a key's first write and
a docs or draft run. Nothing removes a temporary file that a crash left
behind, so each crash can leave one file beside the target until 16 such
files block that process id. `docs/behaviour/durable-files.md` describes
the current behaviour.

Considered: the tempfile crate's `NamedTempFile::persist`, which syncs no
directory and adds a dependency; the atomicwrites crate, which adds one
too; keeping the function in crumbkit's `files` module, which a tool
without crumbkit could not use; `O_NOFOLLOW` on the open, which
`create_new` makes unneeded; random temporary names in place of a counter,
which need a random source and make a leftover file harder to name.
