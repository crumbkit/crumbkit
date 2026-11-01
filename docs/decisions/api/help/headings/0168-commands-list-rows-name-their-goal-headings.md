# ADR-0168. A group's `commands list` row names its goal headings

- Date: 2026-11-01
- Status: accepted
- Supersedes: ADR-0026, which it amends

## Context

A tool's guide declares goal headings for a group's help page through
`Group::new(path, goals)`, and the help pages print them. The JSON of
`commands list` carried no goal, so a program that draws a tool's commands
from that JSON, such as a local web page with a command rail, could not
group them as the help does.

## Decision

- The row of a command whose group in the guide lists goal headings gains
  a `goals` array. Each item is `{"heading": ..., "commands": [...]}`, in
  the order the help page lists the headings, and `commands` holds the
  subcommands' real names in the goal's order.
- `goals` holds what the help page prints. The row and the help page both
  call `goal_sections`, so a hidden subcommand is left out of `commands`
  and a goal whose subcommands are all hidden is left out of `goals`. When
  two groups share a path, the row reads the last one, whose headings help
  shows.
- `goals` lists only the goals that the guide declares. The `Verbs` and
  `Commands` headings that help adds for subcommands that no goal names
  stay out, because a program finds those subcommands among the row's
  children.
- A row of a group without goals, of a group whose goals are all left out,
  and of every leaf has no `goals` key. This amends ADR-0026's rule that
  every row has every field. Under ADR-0007 an optional new field keeps the
  `crumbkit/commands-list/v1` id.
- `goals` is neither a compact nor a long column, as `children`, `examples`
  and `columns` are neither, so `--json --full` and `--fields goals` show
  it and the text table does not change. `--fields goals` writes `null` for
  a row without the field, as it does for any field a row lacks.
- API-1 does not apply, because the change adds no `Document` option, `Row`
  constant, `Guide` setting or `Leaf` setting.

## Consequences

A program groups a tool's commands as its help does from one
`commands list --json --full` run. A tool whose guide declares no goals
prints the same JSON as before.

Considered: a `goal` string on each child row, which loses the order of
the headings; parsing the text of `-h`, which has no stability promise; a
goals list on the root row only; an empty `goals` array on every row;
including the `Verbs` and `Commands` headings.
