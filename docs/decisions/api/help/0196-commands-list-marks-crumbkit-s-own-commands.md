# ADR-0196. `commands list` marks crumbkit's own commands

- Date: 2026-11-10
- Status: accepted. It amends ADR-0026, whose rows did not say who provides a command.

## Context

A program that drives a crumbkit tool, such as a local page over the tool's
commands, wants to tell the commands that crumbkit and its add-on crates
provide, such as `commands list`, `schemas show`, `self usage` and
`aliases list`, from the tool's own. Without a field it guesses from the
top-level group name, so a tool's own command under one of those groups,
such as a tool's `self doctor`, is classed as crumbkit's.

The tool owns its command tree: each tool mounts the `commands`, `schemas`,
`self`, `skills` and `aliases` groups in its own clap types, and crumbkit
never mounts them. The document that a leaf declares is the one place in
the tool record where crumbkit's code, rather than the tool's, decides what
the command is.

## Decision

- `Document` gains a hidden builder, `built_in`, which only crumbkit's
  crates call where they define their documents. `COMMANDS_LIST`,
  `SCHEMAS_LIST`, `SCHEMAS_SHOW`, `SELF_USAGE`, `SELF_FIGURES_LIST`,
  `SELF_RUNS_LIST`, `SELF_RUNS_EXPORT`, `SKILLS_INSTALL`, `SKILLS_CHECK`,
  `SKILLS_PRINT` and `ALIASES_LIST` are marked. The mark survives
  `untyped`, so a leaf's document keeps it.
- Every `commands list` row gains `built_in`, a boolean that is always
  present. It is `true` for a command whose leaf declares a marked document
  or stream, and for a group whose visible commands below it number at
  least one and are all built in. It is `false` for every other row.
- `list` works out a group's mark on the same walk that collects the
  branch, from the marks of its visible subcommands, so no code compares
  group names and the branch is walked once.

## Consequences

The ground for the new `Document` option under the api-1 rule is one
adopter's request for an ordinary need that other programs which drive
crumbkit tools share. The text table keeps its columns, so `built_in`
appears with `--json`, `--full` or `--fields`. A crumbkit command whose leaf
the tool does not declare reads `false`, and crumbkit-test's leaf checks
already require the leaf.

Considered: a list of crumbkit's group names, which misclasses a tool's
own command under those groups; a marker on the clap command, which the
tool builds and crumbkit never sees; a public builder, which would let a
tool mark its own documents as crumbkit's.
