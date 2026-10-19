# ADR-0024. Leaf examples and columns

- Date: 2026-10-09
- Status: amended by ADR-0104

## Context

An agent learns how to run a command fastest from an example, and learns
what a result means from its columns. Both descriptions must come from
one place, so they cannot drift from the code, and a run that prints no
long help should not pay to build them.

## Decision

- A tool declared its leaves in the `Guide`, each naming a command by
  its real path and holding its examples, effects and document. `Leaf`
  holds function pointers for its schemas and so has no type parameter,
  which lets one guide hold leaves of different row types.
- A leaf's `-h` listed its examples as full command lines, and `--help`
  described each example and column.
- A column's description was the doc comment of the row's field, read
  through the JSON Schema, so the doc comment is the only place a column
  is described. `--help` and `check_guide` read the same descriptions.
- `parse` built the schema only for `--help`.
- Command lines printed with `help::quote` for a POSIX shell, the shells
  of the tier-1 platforms.
- `check_guide` parsed each example with clap and required an example for
  every visible command that runs, and a description for every column.

## Consequences

On Windows a pasted example may need other quoting.
`docs/behaviour/help/examples.md` describes the current behaviour.

Considered: a string path such as `Leaf::new("books list")`, a `#[command(after_help)]` attribute per leaf, a separate column-description list beside `Row::COLUMNS`, building the schema on every run.
