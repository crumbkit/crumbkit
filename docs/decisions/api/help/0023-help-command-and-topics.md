# ADR-0023. Help command and topics

- Date: 2026-10-09
- Status: accepted

## Context

clap generates a `help` subcommand that adds a line to every `Commands:`
list and makes `Command::build` copy the tree into it. Tools also have
material that belongs to no single command, such as concepts and exit
codes, and an agent needs one place to find it.

## Decision

- `parse` answered the word `help` itself, at every level, and turned off
  clap's generated subcommand, so the line leaves every list and the
  tree is not copied. It answers before clap parses, so a tool's own
  `help` command still takes the word.
- `<tool> help` listed the root's topics and `<tool> help <TOPIC>` printed
  one. Every other `help` line became the same line with `--help`.
- Topics were declared in the `Guide` as literal text or `include_str!`,
  so the tree holds no topics and no run copies their text.
- crumbkit added its own `exit-codes` topic, last in the list, and
  `check_guide` rejects a tool topic with that name, because crumbkit's
  answers first. A root without subcommands or topics still reads `help`
  as an ordinary word.
- Topics printed as plain text, and the root's help names them on one
  line, so they cost one line of the root's help.

## Consequences

`check_grammar` reserves `help` for asking for help, with no opt-out.
`docs/behaviour/help/README.md` describes the current behaviour.

Considered: the hidden `help` command carrying the topics, clap's `unstable-ext` extension slot, a deferred builder per leaf, a `Help` variant in the tool's enum, `help` only at the root, crumbkit writing its own message for an unknown word, topic files read at runtime, a Topics section with summaries on the root page, keeping clap's `help` for tools without topics, a separate `exit-codes` command, exit codes listed on the root's help page.
