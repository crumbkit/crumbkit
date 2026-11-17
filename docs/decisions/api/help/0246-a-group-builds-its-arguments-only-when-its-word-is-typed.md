# ADR-0246. A group builds its arguments only when its word is typed

- Date: 2026-11-17
- Status: accepted

## Context

`Cli::command()` builds every argument of every group on each run and the run
drops them. A profile of `shelf -h` put about 120,000 of the 149,222
instructions added since 0.4.0 in building the groups `self runs`,
`completion`, `skills` and `self`.

## Decision

- `crumbkit::Lazy<S>` wraps a `#[command(subcommand)]` field. It defers the
  group's subcommands with clap's `Command::defer`. A tool reads the value
  with `into_inner`.
- `Lazy` sets the value name to clap's default, `COMMAND`, when the tool set
  none, so help is unchanged and an unexpanded group is the one with that
  name and no subcommands. It restores the variant's about, help and version
  texts, which a deferred function would otherwise override.
- `crumbkit::reach(command, words)` expands each lazy group the words pass
  through, building each command from the root down and the group last. clap
  copies global arguments into direct children only and builds no command
  twice, so a group built first would miss the root's `--json`. `reach` sets
  `disable_help_subcommand(true)` first.
- `parse`, `Aliases::expand` (now `&mut Command`), `help::extend` and
  crumbkit-test's checks call it or expand every group. A run that types no
  lazy group's word builds nothing.

## Consequences

A tool's own code that reads `Cli::command()` without building it sees a lazy
group without subcommands.

Considered: a macro per group, which does not help a tool's own groups;
building the whole tree in `reach`, which costs what the change removes;
expanding in each tree reader, which repeats the root-first rule.
