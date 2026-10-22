# ADR-0134. A tool gives its own sink the width that a text table fits

- Date: 2026-10-21
- Status: accepted
- Supersedes: ADR-0121, which it amends
- Missed: ADR-0121 gave only `finish` a width, so a tool that writes through its own sink could not fit a table.

## Context

ADR-0121 made a text table narrow its columns to the terminal's width. Only
`finish` gave its sink a width, through a setter private to crumbkit, so
`finish_to` with a tool's own sink never fitted a table, and
`Row::UNCUT_COLUMNS` had no effect there. An adopting tool writes its
results through its own output owner with `Sink::new` and `finish_to`, and
it reads whether standard output is a terminal and how wide it is through
its own environment owner. At a terminal, its long cells were no longer
cut. The tool asked for a public setter in the style of
`Sink::styled_errors`.

## Decision

- `Sink::fitted` is a public setter that takes an `Option<usize>`, and
  `Sink::fitted_width` reads it. `None` means the table prints unfitted. A
  sink from `Sink::new` or `Sink::with_input` has no width, so every
  existing tool and test prints the same bytes.
- The setter checks no terminal, as `Sink::styled` and
  `Sink::styled_errors` check none. The tool decides the width, and the
  guide's terminal detection does not change a width that the tool sets.
- `finish_to` fits a text table to its sink's width by ADR-0121's rule, and
  the output budget still counts the unfitted table. A width narrower than
  the headers and the uncut columns together, such as 0, leaves the table
  unfitted, as a terminal that narrow does.
- A sink that crumbkit borrows from the tool's sink, or that buffers its
  result, keeps the width of the sink it comes from, as it keeps the
  styles.
- JSON, prose, bytes, a row's own text line, help, notes and failures
  print the same bytes whatever the width. `stream_to` keeps a stream's
  table at the widths of its header, because the stream prints each row
  as it arrives.
- The API-1 ground is one adopter's request for an ordinary need, because
  any tool that prints through its own sink at a terminal needs it
  (ADR-0122).

## Consequences

A tool that owns its output streams fits its tables at a terminal as
`finish` does. OUTPUT-1 keeps the table's width as its exception, because
the width still comes from the terminal, which the tool reads itself.
`docs/behaviour/output/terminal.md` describes the current behaviour.

Considered: naming the getter `width`, which would clash with the sink's
own per-role width and read as if every piece of output had one;
`is_fitted`, which returns a bool and loses the number of columns; and
`fitted_width`, which pairs with the setter `fitted` as `is_styled_errors`
pairs with `styled_errors`. Also considered: reading the terminal inside
`finish_to`, which would make a test's output depend on the terminal that
runs it.
