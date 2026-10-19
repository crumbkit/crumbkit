# Write guards

This page covers the guards of a write: a command that replaces data that
another caller may have changed, and a command that writes many items.
[`Effects`](../../../crates/crumbkit/src/help/effects.rs) and
[`Expect`](../../../crates/crumbkit/src/expect.rs) describe the items
themselves.

## Declaring guards

An agent that retries a write, or that writes while another caller writes,
needs to know whether the command replaces data that another caller may have
changed, and what a failed write of many items leaves behind. A leaf declares
both on its `Effects`, beside the MCP properties
([effects](README.md#declaring-effects)).

- `Effects::replacing` is a constructor for a command that replaces existing
  data, such as one that sets a field or saves a whole file. It is
  destructive and not idempotent, because a second run can overwrite a change
  made between the two runs.
- `Effects::all_or_none` declares that a write of many items changes every
  item or none.
- `Effects::per_item` declares that a write of many items changes each item
  it can, reports a result for each, and exits with code 1 when any item
  fails. A read-only command writes nothing, so a `const` that calls either
  builder on `Effects::read_only` fails to compile.

Help prints a `Writes:` line after the `Effects:` line when a command
declares a guard or takes crumbkit's `--expect`, such as
`Writes: replaces existing data; --expect <VERSION> fails with a conflict when the data changed.`
A replacing command without `--expect` says `without a version check`.
`commands list` writes the guards under `writes` as an object with
`replaces`, `expect` and `items`, and writes `null` for a read-only command or
one without guards, so the row keeps its `v1` id. The Markdown reference
prints the same line ([commands list](../help/commands-list.md),
[reference](../help/reference.md)).

Why: ADR-0047.

## `--expect`

A command that replaces data flattens `Expect`, which adds
`--expect <VERSION>` with the clap id `crumbkit_expect`. The tool decides what
a version is, such as a revision number, and prints it from the command that
reads the target. `--expect none` requires that the target does not exist, so
the word `none` is reserved. The option is optional, and it refuses an empty
value as a refused command line.

The handler passes the target's current version, or `None` when it does not
exist, to `Expect::check` before it changes anything. A mismatch fails with
`<tool>::version::conflict`, or the tool's rename of it. The failure exits
with code 1 unless the guide lists the code for another, names the `expected`
and `current` versions as fields, and recovers by running the command that
the tool passes as the read call. A tool that compares versions in its own
way calls `Expect::conflict` directly. The check runs before
`Output::confirm`, so a conflict comes before the question, and a replacing
command still takes `--yes` because it is destructive.

Help and `commands list` read whether a command takes `--expect` from the
clap tree by the id `crumbkit_expect`, so a leaf declares nothing more for it.

Why: ADR-0047, ADR-0056.

## Checks

crumbkit-test's [`check_effects`](../../../crates/crumbkit-test/src/effects.rs)
enforces these rules:

- A replacing leaf without crumbkit's `--expect` fails unless an opt-out from
  `Rule::Expect` (`effects/expect`) covers it.
- A command with its own `--expect` fails unless an opt-out from
  `Rule::ExpectFlag` (`effects/expect-flag`) covers it.
- A leaf that writes, takes many values in a visible positional argument, and
  declares neither `all_or_none` nor `per_item` fails unless an opt-out from
  `Rule::Items` (`effects/items`) covers it. The check reads a positional
  argument as the items, because options with many values more often filter
  or describe the change.
- A read-only leaf that takes crumbkit's `--expect` fails, and so does a
  `per_item` leaf whose document is not a `Batch`, a `Prose` or a stream,
  because those are the documents that report a failure for each item.
  `Leaf::shape` returns the document's `Shape` for this rule. These rules
  have no opt-out. A read-only leaf cannot declare how its items end,
  because that `const` fails to compile.

shelf's `books remove` declares `all_or_none`. reading's `entries set` takes
`--expect` with the entry's current title as its version, and each list's
`edit` is replacing and opts out of `Rule::Expect`, because the editor saves
the whole file.

Why: ADR-0047, ADR-0036.
