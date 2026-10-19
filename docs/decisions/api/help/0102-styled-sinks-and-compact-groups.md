# ADR-0102. Styled sinks and compact groups

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0021, which it amends

## Context

ADR-0021 printed help in colour only through clap's `Error::print`, which
writes to the process's standard output. A tool that parses with
`parse_to` and its own `Sink`, such as a tool that tests its terminal
output or runs in a server, always received plain help, even when its
writer was a terminal.

ADR-0021 also gave a group's `-h` and `--help` the same list, one line for
each subcommand with its description. A group with many subcommands then
has a long `-h`, while a leaf's `-h` already leaves its columns to
`--help`.

## Decision

- A `Sink` records whether its output writer wants styles, through
  `Sink::styled(bool)` and `Sink::is_styled`. The default is no styles.
  `parse_to` writes help to a styled sink with clap's styles, as
  `StyledStr::ansi` renders them, and writes plain text to every other
  sink.
- `parse_from` marks its standard sink as styled when terminal detection
  is on and standard output is a terminal, and then keeps clap's
  `Error::print`. That call honours `NO_COLOR` and the command's
  `ColorChoice`, which a tool's own sink has to honour itself.
- `StyledStr::ansi` exists only with clap's `color` feature. crumbkit's new
  `color` feature turns it on and is off by default, so a binary still
  chooses clap's colour (ADR-0003). Without the feature, a styled sink
  receives plain help.
- clap renders the version without styles, so a styled sink receives the
  same version bytes as a plain one. Topics and the topic list stay plain
  text, because crumbkit builds them without styles.
- `Group::compact` makes a group's `-h` list each goal's subcommand names
  on one line, after the heading and a colon, padded to one column, such as
  `Read:    list, show`. Its `--help` keeps the full list. Commands that no
  goal names follow on a final `Commands:` line.
- clap's template has one slot whose text differs between `-h` and
  `--help`: `before_help` against `before_long_help`. A compact group puts
  its two lists there, so its command loses its own `before_help` and
  `before_long_help`, as a list with `{` already did. `extend` and `parse`
  apply a compact group through the same function, so both print the same
  pages, and `check_guide` and the help weight read those pages unchanged.
- shelf's root group is compact.

## Consequences

`docs/behaviour/help/README.md`, `docs/behaviour/parsing/README.md` and
`docs/behaviour/output/terminal.md` describe the current behaviour.

Considered: crumbkit always enabling clap's `color`, which would choose
colour for every binary; `parse_from` writing `ansi` itself, which would
lose `NO_COLOR` and `ColorChoice`; a colour parameter on `parse_to` in place
of a field on the sink; keeping a compact group's own `before_help` in the
template, which would print the same list in `-h` and `--help`; a separate
template for `-h`, which clap does not offer.
