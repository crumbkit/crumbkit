# ADR-0232. SNAPSHOTS=overwrite is safe with parallel test processes

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
2. A run that skips an example for a missing precondition kept the
   example's `.stderr` file only when the file existed at that moment. A
   second run that provides the precondition and wrote the file between that
   test and the first run's stale scan lost it to the scan. The two runs
   share a directory by design, because the run that provides a
   precondition owns the example's files (ADR-0163).

## Decision

- Every golden file that a crumbkit-test check writes goes through one
  helper. It writes a temporary file in the same directory, named
  `.<file>.<pid>.<n>.tmp`, and renames it over the golden file, through
  crumbkit-durable's `write_atomically`. A reader sees the old file or the
  new one in full. The temporary name ends in `tmp`, which no check treats
  as a golden extension, so a stale scan never deletes or counts it.
- An overwrite compares the bytes itself and writes only a file whose bytes
  differ. snapbox compares and reports in a comparison and never writes.
- A run that skips an example lists the example's `.stderr` name as owned
  whether or not the file exists. A listed name with no file has no effect
  on the stale scan, so the skipping run deletes nothing that the providing
  run writes, whatever the order of the two.
- Two different checks still cannot share a directory, because each deletes
  the other's files as stale. That rule is unchanged from ADR-0004.

## Consequences

Each rewritten file costs a sync and a rename, a few milliseconds, and an
unchanged file costs one read. Two runs of one check that overwrite a
directory at once write the same bytes, so the last rename wins with the same
content.

Considered: a lock on the golden directory held from the first read to the
stale scan, which serialises overwrites but adds a lock file, a timeout and a
failure path for a race that the skip rule removes; one check per directory,
enforced with a marker file, which adds a file to the golden directory.
