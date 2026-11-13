# ADR-0214. A row declares its children as one value

- Date: 2026-11-13
- Status: accepted
- Supersedes: ADR-0120, ADR-0138 and ADR-0182, which it amends
- Missed: ADR-0182 kept `Row::child_lines` and `Row::long_child_lines` as trait methods because they read the row, although a type could still override them with `CHILDREN` at `None`, which compiled and did nothing.

## Context

Under ADR-0182 a row type declared its child lines in three places.
`Row::CHILDREN` held a `Children` record with the field, the indent, the
blank line and the help text. `Row::child_lines` built each row's lines, and
`Row::long_child_lines` built them for `--long`. crumbkit read both methods
only when `CHILDREN` was set, so a type that overrode them and forgot
`CHILDREN` printed no child lines, and nothing reported that its methods were
never called.

## Decision

- `Children` is `Children<R>`, and `Row::CHILDREN` holds an
  `Option<Children<Self>>`. `Children::lines` takes a function
  `fn(&R) -> Vec<String>` that builds a row's lines, and
  `Children::long_lines` takes one that builds them for `--long`.
  `Row::child_lines` and `Row::long_child_lines` are removed.
- Without `lines`, the lines are the items of the field, as the default
  `child_lines` returned them. Without `long_lines`, `--long` prints the
  compact lines, as the default `long_child_lines` did.
- `Children::lines_for` and `Children::long_lines_for` return a row's lines.
- `Children::untyped` returns a `Children<()>` without the functions.
  `LeafDocument::children` returns it, so help, `commands list`, schemas,
  crumbkit-markdown and crumbkit-test read the field, the indent and the help
  text as before.
- `Children` holds function pointers, so it has `Clone`, `Copy` and `Debug`
  written without a bound on the row type, and no `PartialEq`.

## Consequences

A row type cannot declare child lines without the field that holds them. A
type that overrode `child_lines` moves its body into a method or a closure
that it passes to `Children::lines`, as in
`Children::field("parts").lines(Self::parts)`. A closure names the type of
its parameter, as in `.lines(|service: &Self| ..)`, because the chain of
builder methods does not tell the compiler the row type before the closure.
A caller outside crumbkit that asked a row for its lines writes
`R::CHILDREN.map(|children| children.lines_for(row))`.
`docs/behaviour/output/shapes/README.md` describes the current behaviour.

Considered: a separate record without a type parameter for the settings that
help reads, which adds a public type that `Children<()>` already covers; and
keeping the methods beside the functions, which keeps two ways to build the
same lines.
