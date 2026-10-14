# ADR-0026. Branch export

- Date: 2026-10-10
- Status: accepted

## Context

An agent that needs to learn a branch of a tool reads its help page by
page, one call per command. A single command that exports the branch with
its arguments, examples, columns and exit codes saves those calls, and
must not show commands the tool hid.

## Decision

- `<tool> commands list [PATH]...` listed a branch with each command's
  arguments, examples, columns, effects, write guards, form and exit
  codes. It is a resource with a `list` verb that flattens `Page`, so it
  passes the tree checks with no opt-out.
- The tool owned the wiring, so a tool that does not want the export
  leaves it out. The command sits at the root, because the not-found
  recovery runs it there.
- `list` read the run-time tree from `Output::set_command`, built whole,
  so it shows the commands the tool kept at run time. A hidden command
  fails as an unknown one does, so the failure never reveals it.
- Every row had every field, with `null` or an empty list, so a program
  reads every row the same way. The help and version flags are left out,
  because `parse` changes them, and a global argument is listed once.
- The text table showed only path and about, because nested lists would
  print as JSON in a cell. `--json --full` prints the rest.
- crumbkit's exit codes 0, 1 and 2 were left out of each row, because
  they are the same for every command, and the text ends with a note
  that names them.

## Consequences

crumbkit-markdown builds its reference from this JSON document.
`docs/behaviour/help/commands-list.md` describes the current behaviour.

Considered: `schema cli [PATH]` with built-in exemptions in the checks, `parse` adding and answering the command itself, one nested tree document, a `--markdown` flag now.
