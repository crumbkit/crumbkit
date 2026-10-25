# ADR-0151. A row type can space its rows apart

- Date: 2026-10-25
- Status: accepted
- Supersedes: ADR-0120, which it amends
- Missed: ADR-0120 printed each row's child lines directly above the next row, so a long listing of rows with many child lines gave a reader no mark of where one row's block ends and the next begins.

## Context

ADR-0120 prints each row's child lines under the row. An adopting tool
lists services with their processes as child lines, and with `--long`
the listing runs to many lines, because each service has several
processes. The service rows are then hard to find among the child lines.
The tool's earlier renderer printed a blank line between services, and
the tool asked crumbkit to keep it.

## Decision

- `Row` gains the const `SPACED: bool`, which defaults to `false`, so
  every existing row type prints the same bytes.
- When a row type sets it, the text prints a blank line after a row that
  printed child lines, when another row follows. A row without child
  lines is followed by no blank line, because a run of single-line rows
  already reads as a table. The last row that prints is followed by no
  blank line, so the output never ends in one.
- The rule covers every place where the text prints child lines: the
  default table, `--long`, and rows with their own text line. A listing
  that the output budget cuts ends with its last printed row and no blank
  line, and the budget, including `Room`, counts each blank line.
- `--fields` prints no child lines, so it prints no blank line. `--json`
  does not change. A stream does not change, because it prints each item
  as it arrives and cannot know whether another item follows.
- crumbkit-test's `check_guide` adds no check. `SPACED` on a row type
  without `CHILDREN` changes no output, as `CHILD_INDENT` changes none
  there (ADR-0135).
- The API-1 ground is one adopter's request for an ordinary need, because
  any tool that lists items with several lines of detail under each needs
  a mark between the items (ADR-0122).

## Consequences

A row type with long blocks of child lines reads as separate blocks, and
a reader that splits the text on blank lines gets one block per row. A
reader that takes every line after the header as a row or a child line
must skip the blank lines of a spaced row type.
`docs/behaviour/output/shapes/README.md` describes the current behaviour.

Considered: a blank line after every row, which spaces out single-line
rows that need no mark; a blank line after the last row too, which ends
the output in an empty line; a `Document` option, which would let two
commands print the same rows differently although the spacing follows
from the rows' child lines; a separator line such as `--`, which a reader
would mistake for data; and the names `BLANK_LINE_BETWEEN` and
`SEPARATE_ROWS`.
