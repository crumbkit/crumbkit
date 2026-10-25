# Terminal detection

This page covers what changes when a stream is a terminal, how a text table
fits the terminal's width, how a tool turns detection off, and how crumbkit
tests runs at a terminal.
[`Guide::without_terminal_detection`](../../../crates/crumbkit/src/help/terminal.rs)
describes the setting itself.

## What detection changes

A tool's result, failure or note prints the same text whether a stream is a
terminal or a pipe, and whether or not a coding agent runs the tool
(OUTPUT-1). In a result, only a text table's width changes the text, and
its cells' styles add escape codes around it. A golden file of a piped run
therefore pins what a person reads at a terminal wider than the table.

Detection changes only these:

- Help keeps clap's colours when standard output is a terminal. `parse`
  marks its `Sink` as styled then, and a tool that passes its own sink to
  `parse_to` marks it with `Sink::styled` ([help](../help/README.md)).
- A refusal and the group page printed for it keep clap's styles when
  `CLICOLOR_FORCE` is set, or when standard error is a terminal whose
  `TERM` is not `dumb` and `CLICOLOR` is not `0`. They never keep them
  under `NO_COLOR` ([parsing](../parsing/README.md#refused-command-lines)).
- `Output::progress_allowed` is true only when `--json` is off and standard
  error is a terminal ([output](README.md#following-the-callers-format)).
- A picker and `Output::confirm` ask only when standard input and standard
  error are terminals and no agent is detected
  ([effects](../effects/README.md), [pickers](../effects/pickers.md)).
- A text table that `finish` prints fits the width of the terminal that
  standard output is, as [the table's width](#the-tables-width) describes.
- A text table that `finish` prints shows its cells' styles when standard
  output is a terminal that allows colour ([cell styles](styles/README.md)).
- The usage log's `caller` field reads detection. The usage log is not
  output that a caller reads ([usage log](../usage-log/README.md)).

Why: ADR-0049, ADR-0102, ADR-0121, ADR-0131, ADR-0133, ADR-0150.

## The table's width

When standard output is a terminal and the guide keeps detection on,
`finish` reads the terminal's number of columns once, with rustix's
`tcgetwinsize`, and gives it to its sink as the width of the result. A
tool that prints through its own sink sets the width with `Sink::fitted`,
which checks no terminal, and `finish_to` fits the table to it.
`Sink::new` gives no width. A note or a failure has no width. A compact,
`--long` or `--fields` table wider than the width narrows its columns until
it fits:

- A line of the table is as wide as the sum of its column widths plus two
  characters for each separator between columns.
- While a line is wider than the width, the widest column that is wider
  than its header narrows by the excess, but never below the width of its
  header. Of two columns equally wide, the leftmost narrows first. The next
  such column narrows when the table still does not fit.
- A cell wider than its narrowed column shows its first `width - 1`
  characters followed by `…`. Widths count characters, as the rest of the
  table does.
- A column that the row type names in `Row::UNCUT_COLUMNS`, such as an
  identifier that a person copies, never narrows, also in a `--fields`
  table. crumbkit-test's `check_guide` fails a name there that is in
  neither `COLUMNS` nor `LONG_COLUMNS`.
- When the uncut columns and the other columns' headers are wider than
  the width on their own, the table prints unfitted. A table without
  uncut columns prints unfitted when its headers are wider than the width.

The output budget counts the bytes of the unfitted table, so a run at a
terminal shows the same rows as a piped run and the cut line counts the
same rows ([output budget](budget/README.md)).

`--json`, prose, bytes, a row's own text line, help, notes and failures
never change with the width. A stream's table keeps the widths of its
header, because it prints each row as it arrives
([streams](streams.md#text-and-json-lines)). On a platform other than
Unix, `finish` reads no width and a table prints unfitted.

Why: ADR-0121, ADR-0128, ADR-0134.

## Turning detection off

A tool turns detection off with `Guide::without_terminal_detection`, which
takes the tool's reason. crumbkit-test's `check_opt_outs` lists the setting
as `terminal-detection/off` with that reason and its costs: a person at a
terminal is never asked and must type every argument and `--yes`, progress,
help, refusal and cell colours never show, and a text table never fits the terminal's width
([opt-outs](../adoption/opt-outs.md)).
The `Output` that `parse` returns, the `Output` inside a `Stop` and the
`Output` that `Output::new` builds copy the setting from the guide, as they
copy the guide's JSON layout and code renames.

The setting belongs to the tool and has no environment variable. A caller
already turns colour off with `NO_COLOR`, and the colours of help, a refusal
and cells with `TERM=dumb` or `CLICOLOR=0`,
questions off with `--no-input`, and progress off with `--json`.

Why: ADR-0049.

## The pseudo-terminal test

shelf's `tests/terminal.rs` runs each of shelf's examples with standard input
and standard output on one pseudo-terminal and standard error on another,
each 500 columns wide. Standard output without its
escape codes must equal the example's `.txt` golden file, and standard error
its `.stderr` file, or nothing when the example has none
([golden files](../checks/golden-files.md)). A first test checks that help
there has clap's colours, so the suite fails when the run does not reach a
terminal. Another runs `shelf books list --all` 40 columns wide and checks
that the titles are cut and each id is whole.

Three tests run `shelf books list --all`. Its titles are bold, and under
`NO_COLOR` or `TERM=dumb` it prints its golden file exactly (ADR-0150).
Four tests run the refused `shelf books list --limit 101`. Its standard
error carries clap's styles, and without them it equals the `.stderr`
golden file. Under `NO_COLOR`, `TERM=dumb` and `CLICOLOR=0` it equals the
golden file as printed (ADR-0131, ADR-0133).

The test uses rustix's safe `pty` and `termios` functions, which need no
`unsafe` code (RUST-2) and add no crate to the lock file. It runs on macOS
and Linux only, because Windows has no such API without `unsafe` code or a
new crate.

Why: ADR-0049.
