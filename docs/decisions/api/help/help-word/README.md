# ADR-0101. tool help prints the root's help

- Date: 2026-10-20
- Status: amended by ADR-0129
- Supersedes: ADR-0023, which it amends

## Children

- [ADR-0129. help reads the words after it with the command's options](0129-help-reads-option-values.md)

## Context

ADR-0023 made `<tool> help` list the root's topics and `exit-codes`, each
with its summary, and end with `Next: <tool> help <TOPIC>`. A caller who
types `<tool> help` usually wants the tool's help, and the topic list
showed none of the tool's commands. The caller then had to run
`<tool> --help` as well.

The root's help already names every topic on its `Topics:` line, such as
``Topics: catalogue, exit-codes. `shelf help <TOPIC>` prints one.``. The
same rule gives a root that line and lets it answer `help`: the root has
subcommands or the tool declares topics, and it has no command of its own
named `help`. So every tool that answers `help` names its topics in its
root's help, and no topic needs the list to be reached.

## Decision

- `<tool> help`, with no word after `help`, prints what `<tool> --help`
  prints. crumbkit turns the word into `--help`, as it does for every
  other `help` line that names no topic.
- The topic list page and its `Next: <tool> help <TOPIC>` line are removed.
- `<tool> help <TOPIC>`, `<tool> help exit-codes`, `<tool> help <command>`
  and `<command> help` keep the behaviour of ADR-0023.
- `Topic::about` stays, so declaring a topic is unchanged. crumbkit prints
  a topic's summary nowhere, and a tool that lists its topics itself reads
  it from `Topic::about`.

## Consequences

`<tool> help` and `<tool> --help` print one page, so a caller reaches the
commands and the topics from either. shelf's golden file of the topic list
is gone, and its `help` test compares with the root's `--help` golden file.
A run of `<tool> help` on the clap root reads the person's aliases for the
root's help, as `<tool> --help` does.

Considered: keeping the list and adding the commands to it, which repeats
the root's help; printing the root's `-h`, which differs from what
`<tool> help <command>` prints for a command; removing `Topic::about`, which
breaks every tool that declares a topic.
