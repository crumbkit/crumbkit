# Effects

This page covers how a command tells a caller what it changes: the effects
that each leaf declares, dry runs, and the confirmation that a destructive
command asks for. The other pages in this directory cover
[pickers and interactive input](pickers.md) and
[write guards](write-guards.md).
[`Effects`](../../../crates/crumbkit/src/help/effects.rs),
[`DryRun`](../../../crates/crumbkit/src/output/dry_run.rs) and
[`Yes`](../../../crates/crumbkit/src/output/confirm.rs) describe the items
themselves.

## Declaring effects

An agent cannot tell from a command's name whether the command changes
anything, whether a repeat is safe, or whether it destroys data. Each leaf in
the guide therefore declares its `Effects` with `Leaf::effects`, beside its
examples and document.

The effects use the vocabulary of the Model Context Protocol's tool
annotations: read-only, destructive, idempotent and open-world.
`Effects::read_only`, `Effects::additive` and `Effects::destructive` set what
the command changes, and `idempotent` and `open_world` add the other two
properties. A read-only command is idempotent.

Help prints the effects on one line after the examples, such as
`Effects: destructive, idempotent, closed-world.` A command that changes
something names whether it is idempotent, so a missing word never stands for
"no". `commands list` writes the effects as an object under `effects` in each
row, with MCP's four properties as booleans, and writes `null` for a command
without effects ([commands list](../help/commands-list.md)).

A leaf without effects reads as unknown, and an agent must assume that it may
destroy data. crumbkit-test's
[`check_effects`](../../../crates/crumbkit-test/src/effects.rs) requires
effects on every visible leaf, and a tool opts out of `Rule::Effects` for a
command with a reason ([opt-outs](../adoption/opt-outs.md)).

Why: ADR-0039, ADR-0024.

## Dry runs

A command that changes something may flatten `DryRun`, which adds
`--dry-run`. `parse` reads the flag from the deepest command's matches and
records it in `Output`, and a tool that parses with clap alone passes its
`DryRun` to `Output::new`. The handler reads `Output::is_dry_run` and skips
its changes, and `finish` marks the result. The command still validates its
arguments and returns the result that a real run would return.

- A dry run's JSON document holds `"dry_run": true` after `schema`, and a
  real run's document holds no `dry_run` field, so the field keeps every `v1`
  id. A stream marks each JSON line in the same way.
- In text, the line `This was a dry run, so nothing changed.` prints on
  standard error with the other notes. It prints last, after a batch's
  failures, so it is the final line a reader sees
  ([output](../output/README.md#standard-output-and-standard-error)).
- A document with `Envelope::Without` has no envelope to hold the
  field, so its JSON prints unchanged and the line also goes to standard
  error.

`check_effects` fails a read-only command that takes crumbkit's `--dry-run`,
because it has nothing to skip. It also fails a command with its own
`--dry-run`, because `DryRun` gives the flag one meaning in every tool. A
tool that keeps its own flag opts out of `Rule::DryRun` with a reason.

Why: ADR-0039, ADR-0016, ADR-0041.

## Confirmation

A command that destroys data flattens `Yes`, which adds `--yes`, and calls
`Output::confirm` with a question before it changes anything. `confirm`
proceeds at once with `--yes` or on a dry run.

- When a person can answer ([pickers](pickers.md#who-can-answer)), `confirm`
  writes the question with ` [y/N] ` to standard error and proceeds on `y`
  or `yes`. Any other answer fails with `<tool>::confirmation::declined`,
  which has no next step.
- When no person can answer, or the caller passed `--no-input`, `confirm`
  fails with `<tool>::confirmation::needs_yes`. Its recovery runs the words
  that the caller typed with `--yes` added before any `--`, through
  `Call::typed`, so an agent can repeat the command exactly. The
  [`Output`](../../../crates/crumbkit/src/output.rs) that `parse` returns
  holds those words, and so does the one that `Output::new` builds from the
  argv. A typed word that is not UTF-8 leaves the recovery without a next
  step.

`confirm` reads through the [`Sink`](../../../crates/crumbkit/src/sink.rs),
whose third type parameter is the input. `Sink::with_input` takes a reader
and whether the sink is interactive, so a test drives the question in memory.
Only `Sink::standard_with_input` locks standard input, so `finish` never
touches it. A picker asks through the same sink.

`check_effects` fails a destructive command without `--yes`, because an
agent without a terminal could not run it. A tool opts out of `Rule::Yes`
with a reason.

Why: ADR-0039, ADR-0043.
