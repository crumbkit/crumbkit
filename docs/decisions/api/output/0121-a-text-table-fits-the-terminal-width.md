# ADR-0121. A text table fits the terminal's width

- Date: 2026-10-20
- Status: amended by ADR-0134
- Supersedes: ADR-0049, which it amends

## Context

The owner reviewed an adopting tool's listings at a terminal, where each
line of a table wider than the terminal wrapped. Before it adopted
crumbkit, that tool cut its last column to fit. The owner chose that a text
table fits the terminal's width.

## Decision

- When standard output is a terminal, the guide keeps terminal detection
  on and `finish` prints a text table wider than the terminal, crumbkit
  narrows columns until the table fits. rustix's `tcgetwinsize` reads the
  terminal's number of columns on Unix.
- A line is as wide as its column widths plus two characters for each
  separator. While a line is too wide, the widest column that is wider
  than its header narrows by the excess, never below its header's width,
  and the leftmost of equally wide columns narrows first.
- A cut cell shows its first `width - 1` characters followed by `…`.
- `Row::UNCUT_COLUMNS` names the columns that are never cut, such as an
  identifier that a person copies. When those columns and the other
  headers are wider than the terminal on their own, the table prints
  unfitted.
- The output budget counts the unfitted table, so the rows that print never
  depend on the terminal.
- JSON, prose, bytes, a row's own text line, streams, help, notes and
  failures print as before.

## Consequences

OUTPUT-1 lists the table's width as an exception. `finish_to` never fits a
table, and a table on Windows prints unfitted.
`docs/behaviour/output/terminal.md` describes the current behaviour.

Considered: cutting only the last column, as the adopting tool did;
spreading the excess over every column; reading the width from `COLUMNS`.
