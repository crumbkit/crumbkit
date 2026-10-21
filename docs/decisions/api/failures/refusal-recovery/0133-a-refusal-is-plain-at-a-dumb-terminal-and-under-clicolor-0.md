# ADR-0133. A refusal is plain at a dumb terminal and under CLICOLOR=0

- Date: 2026-10-21
- Status: accepted
- Supersedes: ADR-0131, which it amends
- Missed: ADR-0131 left `TERM=dumb` and `CLICOLOR=0` unread, so a refusal was styled where help was plain.

## Context

ADR-0131 styled a refusal and its group page on standard error. The
standard sink decided from `NO_COLOR`, `CLICOLOR_FORCE`, whether standard
error was a terminal, the command's `ColorChoice` and the guide's terminal
detection. Help is coloured by clap, which asks anstream, and anstream also
reads `CLICOLOR` and `TERM`. A person whose terminal sets `TERM=dumb`, or
who sets `CLICOLOR=0`, therefore saw plain help and a styled refusal. The
owner approved reading both variables for a refusal.

anstream 1.0, the version that clap uses here, decides for a stream whose
`ColorChoice` is `Auto` in this order: a non-empty `NO_COLOR` turns colour
off; then a non-empty `CLICOLOR_FORCE` turns it on; then `CLICOLOR=0` turns
it off; then colour shows at a terminal when `TERM` is set and is not
`dumb`, when `CLICOLOR` is set to another value, or when `CI` is set.

## Decision

- The standard sink that `parse`, `parse_from` and `Refused::print` build
  decides in this order. A guide that turns terminal detection off, or the
  command's `ColorChoice::Never`, keeps the refusal plain. A non-empty
  `NO_COLOR` keeps it plain, also with `CLICOLOR_FORCE`. Otherwise a
  `CLICOLOR_FORCE` that is neither empty nor `0` styles it. Otherwise
  `TERM=dumb` or `CLICOLOR=0` keeps it plain. Otherwise the refusal is
  styled only when standard error is a terminal.
- `CLICOLOR_FORCE` beats both `TERM=dumb` and `CLICOLOR=0`, because
  anstream checks it before either of them. Help at a terminal follows the
  same order.
- Only the value `dumb` of `TERM` counts. An unset `TERM` changes nothing
  for a refusal, although anstream keeps help plain then. Only the value
  `0` of `CLICOLOR` counts, so a `CLICOLOR` of `1` does not style a refusal
  at a dumb terminal, and the `CI` variable changes nothing.
- The other differences from help that ADR-0131 records stay. A refusal
  honours `CLICOLOR_FORCE` at a pipe, `NO_COLOR` beats
  `ColorChoice::Always`, and `CLICOLOR_FORCE=0` forces no colour.
- `Sink::styled_errors` and `Sink::is_styled_errors` are unchanged. The
  setter checks no environment variable, so a tool that builds its own
  sink decides as before.
- The change adds no public API. Its ground is the ground of the setter it
  refines, one adopter's ordinary request, which the owner approved
  (ADR-0122).

## Consequences

A refusal at a dumb terminal or under `CLICOLOR=0` prints plain text, and
shelf's pseudo-terminal test checks both cases against the piped golden
file. The sink's module reads four environment variables for a refusal,
each through a small reader that the unit tests call with each value.

Considered: asking anstream for standard error's choice, which would also
copy help's reading of an unset `TERM`, `CI` and `CLICOLOR_FORCE=0`;
letting `TERM=dumb` beat `CLICOLOR_FORCE`, which would keep a refusal plain
where help at the same terminal is coloured.
