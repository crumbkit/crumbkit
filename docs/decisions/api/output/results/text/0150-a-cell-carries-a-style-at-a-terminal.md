# ADR-0150. A table cell carries a style at a terminal

- Date: 2026-10-25
- Status: amended by ADR-0154
- Supersedes: ADR-0049, which it amends

## Context

A tool that lists services, jobs or checks prints a state column whose
values, such as `running`, `failed` and `stopped`, a person scans for the
failures. Many CLIs colour such cells at a terminal. crumbkit printed every
cell plain, and OUTPUT-1 forbade a result that depends on the terminal,
except a text table's width (ADR-0121). A tool that wanted colour had to
write its own table and lose the fields, fitting and budget of `finish`.

## Decision

- `crumbkit::Style` names what a cell means: `Emphasis`, `Dim`, `Success`
  and `Error`. crumbkit chooses how each shows: bold, dim, green and red.
  The enum is `#[non_exhaustive]`, so a style can be added later.
- `Row::cell_style(&self, column)` returns a cell's style and returns
  `None` by default. A row type opts in by overriding it, so no Cargo
  feature gates it.
- A cell carries its style only in the text table that `finish` prints,
  and only when the sink's result wants styles, which
  `Deliver::styles(Role::Result)` reads from the flag that `Sink::styled`
  sets (ADR-0128). The header, JSON, a text line, a stream and a child's
  lines are never styled.
- `finish` sets the flag with the rule that styles a refusal (ADR-0131,
  ADR-0133), applied to standard output: off without detection, with the
  command's `ColorChoice::Never` or under `NO_COLOR`, on under
  `CLICOLOR_FORCE`, even at a pipe, off under `TERM=dumb` or `CLICOLOR=0`,
  and otherwise on when standard output is a terminal.
- Each styled cell is its text between the style's escape code and the
  reset. Padding stays outside, and widths, the cut of a fitted table and
  the output budget count the text alone, so the styled table without its
  escape codes is the piped table.
- The `-` that a missing value prints takes the style of its cell.
- crumbkit writes the ANSI codes itself on every platform. Cell styles need
  neither the `color` feature nor clap's rendering, while a refusal's
  styles need both.

The API-1 ground is an ordinary need that most listing tools have, and
shelf's book table, which shows each title in bold.

OUTPUT-1 keeps its rule that no result depends on the terminal, and lists
the styles of a text table's cells among its exceptions, as ADR-0131 listed
a refusal's styles. A cell's style decorates the text and never changes it,
so a golden file of a piped run still pins what a person reads, and a run
under `NO_COLOR` prints the golden file's bytes exactly.

## Consequences

`docs/behaviour/output/styles/README.md` describes the styles, OUTPUT-1
lists them among its exceptions, and shelf's pseudo-terminal test compares
standard output with each golden file once the escape codes are removed.
`check_opt_outs` lists cell styles among the costs of turning terminal
detection off.

Considered: free colours chosen by the tool, which let tools disagree on
what red means; gating styles behind the `color` feature, which would leave
a row type's styles silent in a default build; styling the whole row, which
cannot mark one failing check among healthy columns.
