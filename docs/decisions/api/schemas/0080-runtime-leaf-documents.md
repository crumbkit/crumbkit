# ADR-0080. Documents of runtime leaves

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0035, which it amends
- Missed: ADR-0035 took the documents from the guide's `const` leaves only, so a command added at run time that prints a declared document had no columns in its help, no columns or form in `commands list`, and no place among the commands that `schemas list` names for the document.

## Context

A tool that adds commands at run time builds a `RuntimeLeaf` for each one.
Such a command prints a document that the tool declares as a constant,
such as reading's `<name> list`, which prints the document of `entries
list`, or one command for each plugin that prints the report of a group.
`RuntimeLeaf` had no document, so help offered every view flag on the
command, including flags its document refuses.

## Decision

- `RuntimeLeaf::document` takes a `Document` constant, as `Leaf::document`
  does, and `Guide::with_runtime_leaves` passes it to the joined leaf.
  Help, `commands list`, `schemas` and crumbkit-test's checks then read the
  command as they read a `const` leaf with that document.
- A document's name stays the path of the static command it was named
  for. A runtime leaf's path differs from it, so `check_opt_outs` lists the
  name as a declaration of each such command, as it does for a `const`
  leaf.
- A runtime leaf does not take the document of a `const` leaf whose path is
  a prefix of its own. `check_guide` refuses a leaf whose command has
  subcommands, so such a parent cannot pass the checks.

## Consequences

`docs/behaviour/help/README.md` describes runtime leaves.

Considered: a runtime leaf that takes the document of the `const` leaf with the longest prefix of its path, a document id built at run time, a `stream` setting on `RuntimeLeaf` before a tool needs one.
