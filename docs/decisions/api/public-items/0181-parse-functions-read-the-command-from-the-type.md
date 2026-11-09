# ADR-0181. Parse functions read the command from the type

- Date: 2026-11-09
- Status: accepted
- Supersedes: ADR-0100, which it amends

## Context

`parse`, `parse_from` and `parse_to` took a `Command` beside the type
parameter `C`. Every caller passed `Cli::command()` for `C = Cli`, and
`parse::<A>(B::command(), ..)` compiled although it could only be a bug. The
functions named their other type parameters, so a caller with a turbofish
wrote a placeholder for each one, as in `parse_to::<Cli, _, _, _, _>`.

ADR-0100 gave each of the three functions a `try_` form. Only `try_parse_to`
is called from outside crumbkit, and one of its callers parses against a tree
that it builds at run time, which differs from what the parsed type derives.
`try_parse` and `try_parse_from` differed from `try_parse_to` by a private
flag that says the sink is over the standard streams.

## Decision

- `parse`, `parse_from` and `parse_to` require `C: CommandFactory +
  FromArgMatches` and build the command with `C::command()`. They take no
  `Command` argument.
- `try_parse_to` keeps its `Command` argument and requires only
  `C: FromArgMatches`, so a tool that builds its tree at run time passes it.
- The parameters that a caller never names are `impl Trait`, so a turbofish
  names `C` alone.
- `try_parse` and `try_parse_from` no longer exist. `Sink::standard` makes a
  sink that says it is over the standard streams, as a private property.
  `try_parse_to` with that sink reads terminals as `parse` does: help follows
  the tool record's terminal detection and prints through clap, and the picker
  uses the process's streams.

## Consequences

A tool's `main` writes `crumbkit::parse::<Cli>(&TOOL)`. A tool that needs its
`Command` before parsing, such as for alias expansion, builds it again for
itself, and each run builds the tree twice.

A wrapper type that keeps clap's matches derives no `CommandFactory`, so it
parses through `try_parse_to` and names the command there.

`complete_with` and `complete_with_words` stay. They declare the process's
completion variable, which `events::observe` reads, and `complete_to` does
not, because a sink over in-memory writers would then declare a variable for
the whole process.

Considered: a `Command` argument on every function, which keeps the bug that
`A` and `B` can differ; a second type parameter for a separate command type,
which adds a placeholder that callers would again write; keeping `try_parse`
and `try_parse_from`, which have no callers and differ only by the private
flag; a public `standard` setter on `Sink`, which would let a sink over a
buffer claim to be the process's streams.
