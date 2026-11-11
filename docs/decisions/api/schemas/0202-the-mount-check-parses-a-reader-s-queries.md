# ADR-0202. The mount check parses a reader's queries

- Date: 2026-11-11
- Status: accepted
- Supersedes: ADR-0035, which it amends

## Context

A program that reads a crumbkit tool, such as flightdeck, starts the tool
with fixed command lines and reads what they print. It runs `commands list`,
`schemas list`, `self figures list` and `self runs list` with `--json --full
--all --no-input`, `self usage` with `--json --full --no-input`, and
`schemas show` with `--json --no-input -- <ID>...`.

A tool whose root arguments did not flatten `NoInput` exited with 2 on
`--no-input`, so the program could not load it. The tool still passed every
check of crumbkit-test, because `check_mounts` read only the path of each
command. One adopter found this and wrote its own test that ran each query
against its built binary. Every other tool would have needed a copy of that
test, and the list of queries existed only in the reading program.

## Decision

- crumbkit gains the public module `queries`. It holds the `Query` type,
  with a command's fixed path and the flags that a reader passes, one
  constant for each of the six queries above, and `QUERIES`, the list of
  all six. `Query::words` returns the argument vector after the program,
  and `Query::words_with_ids` returns it with the ids that `schemas show`
  takes after `--`. The reading program builds its command lines from
  this list.
- `check_mounts` parses each query of `QUERIES` whose command the tool's
  `clap::Command` holds, with clap, as the binary would. It fails when clap
  refuses a query, and the message names each flag the command does not
  take with the type that adds it: `NoInput` for `--no-input`, `ViewFlags`
  for `--json` and `--full`, and `Page` for `--all`. When clap refuses the
  query for another reason, the message names clap's error kind.
- A query whose command the tree does not hold is skipped. The rule has no
  opt-out.
- `--version` and `--exclude-program` stay in the reading program. clap
  provides `--version`, and a subtree passed to a check has no version. A
  program appends `--exclude-program` only for a tool whose usage reports
  take it.

## Consequences

Every tool that calls `check_mounts`, by hand or through `conventions!`,
gets the rule without an edit. A tool that mounts one of the commands
without one of the flags fails `check_mounts` after it upgrades, until it
flattens the named type. cargo-crumbkit was such a tool, and it now flattens
`NoInput`. A tree that mounts none of the commands, such as a subtree that a
team checks on its own, still passes. The queries live in crumbkit and not
in crumbkit-test, because a reading program depends on crumbkit as an
ordinary dependency and has no reason to depend on a test crate.

Considered: a new `check_queries` function and `Check` variant, which a
tool that calls the checks by hand would have to add; failing a tool that
does not mount `commands list` or the schema commands, which would fail a
subtree; an opt-out for a tool that cannot take a flag, which has no known
user; constants beside each command in `commands` and `schemas`, which
leaves the `self` queries without a home, because crumbkit-usage owns those
commands and a reading program does not depend on it.
