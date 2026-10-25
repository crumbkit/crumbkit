# Cell styles

This page covers the styles that a row type gives the cells of a text table,
when they show, and what they leave unchanged.
[`Style`](../../../../crates/crumbkit/src/style.rs) and
[`Row::cell_style`](../../../../crates/crumbkit/src/row.rs) describe the API.

## Giving a cell a style

A row type overrides `Row::cell_style(&self, column)` and returns a `Style`
for each cell it marks, or `None`. The column is the field's name, as in
`Row::COLUMNS`. A row type that does not override it prints every cell
plain.

| Style | Meaning | At a terminal |
| --- | --- | --- |
| `Emphasis` | The cell a person should notice first, such as a name | bold |
| `Dim` | A cell of less interest, such as `stopped` | dim |
| `Success` | Something works or finished, such as `running` | green |
| `Error` | A failure or a problem, such as `failed` | bold red |

A problem cell takes the bold red of the `error:` label that starts a
refusal, so a failure in a table looks like a failure on standard error.

shelf's book table shows each title in bold
([`books.rs`](../../../../examples/shelf/src/books.rs)).

## When styles show

The text table that `finish` prints shows its cells' styles when all of
these hold, checked in this order:

1. The guide keeps terminal detection on
   ([terminal detection](../terminal.md#turning-detection-off)).
2. The command's `ColorChoice` is not `Never`.
3. `NO_COLOR` is unset or empty.
4. `CLICOLOR_FORCE` is set to a value other than empty or `0`, which shows
   the styles even when standard output is a pipe, as it shows a refusal's
   styles (ADR-0133). Otherwise `TERM` is not `dumb`, `CLICOLOR` is not
   `0`, and standard output is a terminal.

This is the rule that styles a refusal on standard error, applied to
standard output ([parsing](../../parsing/README.md#refused-command-lines)).
A tool that passes its own sink to `finish_to` shows the styles when it
marks the sink with `Sink::styled`, the same flag that keeps help's colours.

## What styles leave unchanged

- A styled cell is its text between the style's escape code and the reset
  code `ESC[0m`. The padding that aligns the next column follows the reset.
- The `-` that a missing value prints takes the style of its cell.
- Column widths, the cut of a table that fits the terminal's width and the
  [output budget](../budget/README.md) count the text alone. The styled
  table without its escape codes is the table that a pipe receives.
- The header, `--json`, a row's text line, a stream and a child process's
  lines never carry styles.

Why: ADR-0128, ADR-0131, ADR-0133, ADR-0150, ADR-0154.
