# ADR-0231. SNAPSHOTS=overwrite is safe with parallel test processes

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0004, which it amends

## Context

An adopter ran the example check with `SNAPSHOTS=overwrite` under
cargo-nextest, which starts each test in its own process and runs many at
once. The golden files came out wrong, and only one job at a time gave
correct files. Two causes were found, and a test reproduces each.

1. snapbox writes a golden file with `std::fs::write`, which truncates the
   file and then writes it. A check in another process that reads the file
   between those two steps sees an empty or partial file.
2. A check that deletes stale golden files lists the directory once, and a
   run that skips an example for a missing precondition keeps the example's
   `.stderr` file only when the file exists at that moment. A second run
   that provides the precondition and writes the file between those two
   reads loses it to the first run's scan. The two runs share a directory
   by design, because the run that provides a precondition owns the
   example's files (ADR-0163).

## Decision

- Every golden file that a crumbkit-test check writes goes through one
  helper. It writes a temporary file in the same directory, named
  `.<file>.<pid>.<n>.tmp`, and renames it over the golden file, through
  crumbkit-durable's `write_atomically`. A reader sees the old file or the
  new one in full. The temporary name ends in `tmp`, which no check treats
  as a golden extension, so a stale scan never deletes or counts it.
- An overwrite compares the bytes itself and writes only a file whose bytes
  differ. snapbox compares and reports in a comparison and never writes.
- A check that deletes stale files, `check_help_goldens`, `check_examples`
  and `check_schema_goldens`, holds a lock on its golden directory from its
  first read to the end of its stale scan when `SNAPSHOTS=overwrite` is
  set. Two overwrites of one directory then run one after the other, and
  the second sees the files of the first. The lock file is in the system's
  temporary directory, named by a hash of the directory's absolute path, so
  the golden directory holds no extra file. A comparison takes no lock.
- Two different checks still cannot share a directory, because each deletes
  the other's files as stale. That rule is unchanged from ADR-0004.

## Consequences

An overwrite waits for another overwrite of the same directory, for up to
120 seconds, and then fails with a problem that names the directory. Each
rewritten file costs a sync and a rename, a few milliseconds, and an
unchanged file costs one read.

Considered: one check per directory, enforced with a marker file, which
leaves the skip and stale reads of two runs of one check unordered and adds
a file to the golden directory; keeping snapbox's overwrite and only
locking, which still lets a reader of another check see a partial file.
