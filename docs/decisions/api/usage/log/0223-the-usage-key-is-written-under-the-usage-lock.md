# ADR-0223. The usage key is written under the usage lock

- Date: 2026-11-14
- Status: accepted
- Supersedes: ADR-0029, which it amends

## Context

ADR-0029 linked a new usage key into place with `hard_link`, so the key
file either held a whole key or did not exist, and replaced a key of the
wrong length with a rename and no lock. Two runs that found the key
damaged at the same moment could each rename a key into place, and the
run whose key was overwritten hashed its line with a key that no file
keeps. On a file system without hard links, such as FAT on Windows, the
link failed, so every run dropped its line. crumbkit-durable now owns
replacing a file and taking a file lock (ADR-0221, ADR-0222).

## Decision

- A run that finds `usage.key` missing, or of the wrong length, takes the
  `PathLock` on `usage.lock` that an append takes, with the same 100 ms
  wait, and reads the key again under the lock.
- When the second read finds a 16-byte key, the run keeps it. Otherwise
  the run writes a new key with `write_atomically` and `Mode::Private`,
  so the key file holds the old key or the new one, never part of one.
- The run drops the lock before its append takes it again, because a
  `PathLock` is not re-entrant and the run would wait on itself.
- A run that cannot take the lock or write the key writes no line, as on
  any other failure to read or create the key.

## Consequences

Two runs that find the key missing or damaged at once end with the same
key, and the log works on a file system without hard links. A run that
finds no 16-byte key takes the lock twice, which costs one more open and
lock; a run that reads a good key takes no extra lock. clippy refuses
`std::fs::rename`, `std::fs::copy`, `std::fs::hard_link` and the
`std::fs::File` lock methods outside crumbkit-durable.

Considered: keeping `hard_link` for the first key and adding a lock only
around a replacement, which keeps the FAT failure; one lock held from the
key read through the append, which holds the lock while the run builds
its line; a key file created with `create_new` and then written, which a
crash can leave short.
