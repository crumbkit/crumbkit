# ADR-0109. Refused reports the path, the flags and an option's value

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0100 and ADR-0107, which it amends
- Missed: ADR-0107 gave `Refused` clap's error and missed that a tool which prints its own refusal or records the refused run also needs the deepest command's path, the flags typed and a global option's value.

## Context

A tool that prints a refusal of its own names the deepest command's help,
such as `tool status --help`. A tool that records each refused run in a
history of its own records the flags that the caller typed and the value
of a global option such as `--format`. `Refused` holds the path that the
walk reached, the words that crumbkit gave clap and the built command, but
it returned none of the three, so a tool read the words again and had to
know how clap reads option values.

## Decision

- `Refused::path` returns the stored path, which is empty at the root.
- `Refused::flags` returns the clap ids of the flags in `Refused::parsed`,
  in the order typed, as the walk's reader reads them.
- `Refused::value` returns the last value that `Refused::parsed` gives the
  option with a clap id, from the next word or from the flag's own word,
  for the deepest command's own option or a global option above it.

The reader gains the option that each value belongs to. The getters read
the words at call time, so `Refused` stores nothing new.

## Consequences

A tool reads the three without reading the words itself. A value comes
back as one word, so a value delimiter is not applied. `flags` and `value`
read the words that crumbkit gave clap, so the word `help` reports clap's
help id.

Considered: storing the flags and values in `Refused` at refusal time,
which costs every refusal for a getter that few tools call; returning
clap's `ArgMatches`, which clap does not produce for a refused line.
