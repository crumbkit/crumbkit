# ADR-0154. An error cell is bold red

- Date: 2026-10-25
- Status: accepted
- Supersedes: ADR-0150, which it amends
- Missed: ADR-0150 chose plain red for `Style::Error` without comparing it with the `error:` label of a refusal, which clap shows in bold red, so a failure in a table looked weaker than a failure on standard error.

## Context

ADR-0150 gave a text table's cells four styles and showed `Style::Error`
in red. A refusal on standard error starts with an `error:` label in bold
red, which is clap's default error style. A person who has learned from
that label that bold red marks a failure scans a table for the same mark.
A plain red cell beside bold names and green states drew less of the eye
than the problem it reports.

## Decision

- `Style::Error` shows in bold red at a terminal: `ESC[1m` and then
  `ESC[31m` before the cell's text, and the reset `ESC[0m` after it. This
  is the style of clap's `error:` label.
- The other three styles, the rule that decides when styles show, and the
  rule that widths and the output budget count the text alone are
  unchanged.

The API-1 ground is the one ADR-0150 named: an ordinary need that most
listing tools share. This change adds no option.

## Consequences

`docs/behaviour/output/styles/README.md` lists bold red for `Error`. A
golden file of a piped run is unchanged, because a pipe receives no styles.
A tool that pinned the escape codes of an error cell in its own tests
updates them.

Considered: keeping plain red, which leaves a table's failures weaker than a
refusal's label; a separate `Style::Problem` in bold red beside a plain red
`Error`, which asks an author to choose between two styles that mean the
same thing; reading the error style from the command's clap `Styles`, which
would tie a table cell to clap's rendering and the `color` feature that
ADR-0150 kept cell styles free of.
