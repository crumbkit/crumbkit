# ADR-0100. try_parse returns a refusal unprinted

- Date: 2026-10-20
- Status: amended by ADR-0107, ADR-0109, ADR-0125 and ADR-0160
- Supersedes: ADR-0002, which it amends

## Context

ADR-0002 made `parse` return a `Stop` after it printed help, the version or
a refusal. A tool could read the exit code and `--json` from the `Stop`,
but the refusal had already printed by then.

A tool outside this repository prints its failures in a document of its
own and resolves aliases that its users define. When a command line names
such an alias, clap refuses it, because the alias is no command. The tool
needs clap's refusal and the refused words before anything prints, so it
can try the alias and parse again, or print the refusal in its own
document. `parse` gave it neither.

crumbkit also rewrites a command line before clap reads it. It turns the
word `help` into `--help`, so `tool help nonsense` reaches clap as `tool
nonsense --help`, and it replaces a dashed form with the command's own
name. A tool that reads only the rewritten words cannot tell that the caller
typed `help`, and a tool that reads only the typed words cannot tell which
word clap refused.

API-1 covers `Document` options, `Row` constants, `Guide` settings and
`Leaf` settings. A parse function is none of these, so API-1 does not
decide it. The tool outside this repository is the caller, as it was for
`Stop::number` in ADR-0089.

## Decision

- `try_parse`, `try_parse_from` and `try_parse_to` take the arguments of
  `parse`, `parse_from` and `parse_to` and return `Result<(C, Output),
  Unparsed>`.
- `Unparsed` is a `#[non_exhaustive]` enum. `Unparsed::Stopped` holds the
  `Stop` after crumbkit printed help, the version or a topic.
  `Unparsed::Refused` holds a `Refused` after crumbkit printed nothing.
  `Unparsed::print` and `Unparsed::print_to` print a refusal and return the
  `Stop` of either variant, so a tool's wildcard arm has one call to make.
- `Refused` owns the refusal, the `Output` as far as the walk found it, the
  typed words and the parsed words. Its getters are `message`, `code`,
  `number`, `recovery`, `output`, `typed`, `parsed` and `culprit`. `code`
  is the code after the guide's renames. `culprit` reads clap's error
  context for an unknown subcommand, a refused value or an unexpected word
  or flag, and names the flag for a refused view flag.
- `Refused::print` and `Refused::print_to` take the `Refused` by value,
  print it as `parse` printed it and return the `Stop`. Both are
  `#[must_use]`.
- `parse`, `parse_from` and `parse_to` are the `try_` functions followed by
  printing the refusal, so what they print is unchanged.
- The `refused` event, and `exited` for a refused view flag, fire when the
  refusal prints. With the `tracing` feature, the refusal keeps the
  `crumbkit.parse` span and records its code and exit code there when it
  prints.

## Consequences

A tool can resolve its own aliases after a refusal, and print a refusal in
its own document, without reading clap's rendered text. Printing consumes
the refusal, so a run cannot print it twice, and a compile_fail doctest
holds that. A tool that answers a refusal itself and never prints it
reports no `refused` event to the usage log. The tool reports its exit
through `events::ended`.

A `Refused` keeps the typed words. crumbkit copies them only on a run that
changed a word, by a dashed form, the word `help` or a picker, and copies
the parsed words again only on a refusal, so a run that parses allocates
no extra copy.

`Output::new` still prints its own refusal of a view flag (ADR-0093). A
tool that parses with clap alone has no `try_` form of it.

Considered: a `parse` option that turns printing off, which still returns a
`Stop` that holds no refusal; returning the clap error alone, which loses
crumbkit's message, recovery and the typed words; a `Refused` that
implements `Failure`, whose `code` would answer before the guide's renames
while `Refused::code` answers after them; printing through a shared
reference, which would let a run print one refusal twice.
