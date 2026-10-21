# ADR-0135. A row type chooses the indent of its child lines

- Date: 2026-10-21
- Status: amended by ADR-0137
- Supersedes: ADR-0120, which it amends
- Missed: ADR-0120 fixed the indent of child lines at two spaces, so a tool whose child line holds an identifier as wide as the terminal could not keep that line from wrapping.

## Context

ADR-0120 prints each row's child lines under the row, indented by two
spaces. An adopting tool lists records whose reference is an identifier of
79 to 82 characters that a reader copies whole into another command. The
tool prints the reference as the row's one child line, so the other
columns fit the terminal through `Sink::fitted`. Its earlier renderer
printed the identifier at column 1, so every other line fit in 80 columns.
With the two-space indent, the identifier's line is two characters wider
and wraps at 80 columns. The tool asked to choose the indent, and it needs
an indent of 0.

## Decision

- `Row` gained the const `CHILD_INDENT: u8`, the number of spaces before
  each child line, which defaults to 2. Every existing row type prints the
  same bytes.
- The type is `u8`, so the indent is at most 255 spaces, and the compiler
  rejects a larger literal. crumbkit therefore builds no line of more
  than 255 spaces from the constant and needs no runtime check.
- crumbkit-test's `check_guide` adds no check. An indent on a row type
  without `CHILDREN` changes no output, as `child_lines` without
  `CHILDREN` changes none, and an indent of up to 255 spaces only widens
  the child lines, whose bytes the output budget counts.
- Every place that prints child lines uses the constant: the table of
  `finish` and `finish_to`, `--long`, a stream, and rows with their own
  text line. The output budget counts the child lines as printed, with
  their indent. A fitted table never cuts a child line.
- Escaping, JSON, `--fields` and help do not change.
- The API-1 ground is one adopter's request for an ordinary need, because
  any tool that prints an identifier that a reader copies whole on its own
  line needs that line as narrow as it can be (ADR-0122).

## Consequences

With an indent of 0, a child line starts at column 1 as a table line
does, so a reader tells the two apart only by the child line's content.
The tool's doc comment on the children field, which `check_guide`
requires and the document's schema carries, should describe that line.
Help still calls the child lines indented whatever the indent.
`docs/behaviour/output/shapes/README.md` describes the current behaviour.

Considered: `INDENT_CHILDREN: bool`, which offers only two spaces or none;
an indent string such as `"  "`, which could hold a tab or a mark that a
reader cannot count and that crumbkit would have to escape; `usize`,
whose largest values crumbkit cannot allocate; and the names
`CHILDREN_INDENT` and `CHILD_LINE_INDENT`. `CHILD_INDENT` sits beside
`CHILDREN` and `child_lines` and says what it indents.
