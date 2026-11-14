# ADR-0222. A path lock checks that its file is still at the path

- Date: 2026-11-14
- Status: accepted
- Supersedes: ADR-0221, which it amends
- Missed: ADR-0221 kept crumbkit-durable to the standard library and to file writes, and missed the file locks that the same tools take, whose open needs `O_NOFOLLOW`, which std does not name.

## Context

Five lock implementations in the tools that use crumbkit differ, and none
checks, after it takes the lock, that the file it locked is still the file
at the path. A holder that waited on a lock file that another program then
replaced or removed locks a file that no name reaches, while another holder
locks the new file at the path, and both act as if they held the lock.

## Decision

- crumbkit-durable holds `PathLock`, an exclusive lock on a lock file that
  the caller names for the lock alone. crumbkit never renames, truncates or
  removes a lock file.
- `PathLock::acquire_unless(path, deadline, give_up)` opens the file with
  write and create, `0600` and `O_NOFOLLOW` on Unix, and calls `try_lock`.
  While the lock is held it calls `give_up`, returns `Busy` when that
  returns true or the deadline has passed, and otherwise sleeps 5 ms, or
  what is left before the deadline, and tries again. `acquire` gives up
  never.
- Once it holds the lock it compares the device and inode of the open file
  with those of the file at the path. When they differ or the path is gone,
  it releases the lock and opens the path again, whatever the deadline, at
  most 3 times, and then returns `Busy`.
- `still_current` compares the path's identity again, so a holder learns
  that another program removed or replaced the file while it held it.
- On Windows the lock file opens without `FILE_SHARE_DELETE`, so no process
  removes or replaces it while it is open, and the identity check does not
  run.
- The crate depends on rustix's `fs` feature on Unix, for the `O_NOFOLLOW`
  flag. tempfile already brings it into the lock file, so it adds no crate
  to a build. The crate still depends on no other crumbkit crate.

## Consequences

An uncontended acquire makes four system calls, the open, `flock`, `fstat`
and `lstat`, two more than a lock that skips the check. Another program
can still remove the lock file while a holder has it on Unix, and a second
acquirer then locks a new file, so a holder about to change what the lock
guards calls `still_current` first. A lock is not re-entrant within a
process, and a child that `fork` makes without `exec` keeps the lock until
it exits. `docs/behaviour/durable-files.md` describes the current
behaviour.

Considered: an owner check on the lock file, which closes no path that the
directory's permissions leave open and would refuse a tool run under sudo;
removing the lock file on release, which opens the race this check closes;
`libc` for `O_NOFOLLOW`, which adds a direct dependency where rustix is
already in the lock file.
