# ADR-0129. help reads the words after it with the command's options

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0101, which it amends
- Missed: ADR-0101 kept counting every word after `help` that does not start with `-` as a word that names something, so the separate value of an option after a topic's name read as a second name and `<tool> help <TOPIC> --name value` was refused.

## Context

`<tool> help <TOPIC>` printed the topic only when exactly one word after
`help` named something. crumbkit counted as such a word every word before
`--` that did not start with `-`. The separate value of an option therefore
counted too. An adopting tool has a global option that takes a value, and
`<tool> help <TOPIC> --name value` was refused, because `value` read as a
second name, no topic matched, and clap refused the line that crumbkit
built from it. The same line with `--name=value` printed the topic.

The request for long help used the same count, so `--help` went after an
option's value. clap read that line correctly, but the rule differed from
how the walk before parsing reads the same words.

The walk already reads each word with the options of the command it has
reached and the global options of the commands above it, so the value of
an option names no command.

## Decision

- crumbkit reads the words after `help` with the walk's word reader. The
  separate value of an option names nothing, a word that starts with `-`
  names nothing, and `--` ends the reading, so the words after it stay as
  typed.
- `<tool> help <TOPIC> --name value` prints the topic, as
  `<tool> help <TOPIC> --name=value` already did.
- `<tool> help --name value` prints the root's long help, as `<tool> help`
  does, and so does `<tool> help --name <TOPIC>`, because the topic's name
  is the option's value there.
- A `help` line that names no topic becomes a request for `--help`, which
  goes directly after the last word that names something. On a line that
  ends with an option's separate value, `--help` now goes before the option.
- crumbkit adds no option, constant or setting for this.

## Consequences

The words after `help`, the walk, `command_path` and the search for the
word `help` read the typed words by one rule. The reader's limits apply
here too: it misses the arguments of `Command::defer` and the prefixes that
`infer_long_args` accepts, and reads such a word as a flag without values.

A topic still prints as text when a flag such as `--json` follows its name.

Considered: skipping the word after every word that starts with `--`, which
misreads a flag without a value; reading the words with clap itself, which
refuses the line before crumbkit could print the topic; keeping the count
and documenting the `--name=value` form, which leaves the ordinary form
refused.
