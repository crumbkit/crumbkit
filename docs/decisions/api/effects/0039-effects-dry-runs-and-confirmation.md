# ADR-0039. Effects, dry runs and confirmation

- Date: 2026-10-12
- Status: accepted

## Context

An agent cannot tell from a command's name whether the command changes
anything, whether a repeat is safe, or whether it destroys data. An agent
also has no terminal to answer a confirmation question, and a person
expects a question before data is destroyed.

## Decision

- Each leaf declared its `Effects` in the guide, in the vocabulary of the
  Model Context Protocol's tool annotations: read-only, destructive,
  idempotent and open-world. An agent that knows those hints needs no
  second vocabulary. Help and `commands list` print the effects, and a
  command that changes something always names whether it is idempotent,
  so a missing word never stands for "no".
- A leaf without effects reads as unknown, and `check_effects` requires
  effects on every visible leaf unless the tool opts out with a reason.
- A `DryRun` argument added `--dry-run`, so the flag has one meaning in
  every tool. A dry run still validates its arguments and returns the
  result a real run would return. The JSON holds `"dry_run": true` only
  on a dry run, so every `v1` id keeps its schema, and the text note
  prints last on standard error, so it is the final line a reader sees.
- A destructive command flattened `Yes` and called `Output::confirm`
  before changing anything. A person at a terminal answers a `[y/N]`
  question. When no person can answer, `confirm` fails with
  `crumbkit::confirmation::needs_yes`, whose recovery repeats the caller's
  typed words with `--yes` added, so an agent can repeat the command
  exactly.
- `confirm` read through the `Sink`, so a test drives the question in
  memory, and only the standard sink with input locks standard input.
- `check_effects` failed a read-only command with `--dry-run`, a command
  with its own `--dry-run` and a destructive command without `--yes`,
  each with an opt-out for a stated reason.

## Consequences

`parse` keeps the caller's typed words in `Parsed` for the `--yes`
recovery. `docs/behaviour/effects/README.md` describes the current
behaviour.

Considered: effects in a new type outside the guide, an undeclared leaf read as read-only, an undeclared leaf read as destructive, crumbkit's own vocabulary for effects, an effects column in the text of `commands list`, the dry-run line on standard output in text, the dry-run line before a batch's failures, the dry-run line on standard error with `--json` for every document, no text marker for a dry run, a `dry_run` field in every document, exit code 2 for a missing `--yes`, a tool-built `Call` for the `--yes` recovery, the typed words kept in a thread-local, a separate prompt type outside the sink, the `--yes` rule in `check_guide`.
