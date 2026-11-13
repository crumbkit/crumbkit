# ADR-0219. A call names a set flag

- Date: 2026-11-13
- Status: accepted

## Context

A `Recovery::Terminal` call must be concrete, because a terminal runs the
call exactly as it prints. `Call` added an argument in three ways: `arg` as
a placeholder, `value` with a concrete value and `all` with every value of a
field. `Call::arg` on a flag that takes no value rendered the flag's name,
such as `--all`, because that is clap's usage form of a flag, but the call
still held a placeholder and so had no words to run or parse. `Call::value`
on a flag failed with `CallError::TakesNoValue`. A Terminal call could
therefore not include a flag, and an adopting tool whose login command needs
`--password-auth` printed "no next step" in place of the command. The same
gap kept crumbkit from parsing a refusal alternative that named a flag
through `Call::arg`, such as the reading example's `--all`.

## Decision

- `Call::flag(id)` adds the flag whose clap id is `id`, set, held as
  `CallArg::Flag`. It renders as `--long`, or as `-s` when the flag has no
  long name, and is one word of the call's line.
- Rendering looks the flag up as it looks up every argument, so a missing or
  hidden flag fails as it does for `arg` and `value`. An argument that takes
  a value fails with the new `CallError::TakesAValue`.
- A call whose arguments are all values and set flags has words, so a
  Terminal call that holds one prints its command, and a refusal alternative
  that holds one is parsed before crumbkit offers it.
- `Call::arg` keeps its meaning: a flag added with it is a placeholder. A
  Run call that renders `--all` through `arg` prints as before, and a
  Terminal call that does so still offers no step.

## Consequences

The reading example's `every_entry` uses `flag("all")`, so its refusal
alternatives are now parsed. Its printed output does not change. No existing
call changes its output, because no call used `flag` before. A flag that
counts occurrences, such as `-vv`, takes one `flag` per occurrence.

Considered: giving `Call::arg` on a flag that takes no value concrete words,
which needs no new method but makes a Terminal call that prints "no next
step" today start to print its command, and so changes what `arg` means.
