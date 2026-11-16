# ADR-0228. Aliases come from ordered sources

- Date: 2026-11-16
- Status: accepted
- Supersedes: ADR-0027, which it amends

## Context

crumbkit-aliases read one `aliases.toml`, replaced only the first word after
the tool's name, and listed aliases through a hidden command. A tool that also
keeps shortcuts of its own, or entries that live somewhere other than the
person's file, needed a separate expander for each source. Its shortcuts could
not be keys of several words, such as `wt ls`, and they did not appear in help,
which named only personal aliases. The owner approved the design on 2026-11-16 with
three answers: the first listed source wins, aliases from shipped sources appear
in help by default with a per-source opt-out, and expansion happens once per run.

## Decision

- An `Aliases` value holds an ordered list of `Source`s. A source is a file, a
  table in the tool's code, or a lazy loader, and it has a kind: `Personal` or
  `Shipped`. The first source that has a matching entry wins, so a tool lists
  the person's file first. The alternative of picking the longest key across
  all sources makes the order hard to read.
- A key is one or more words, and a value is a list of one or more words. The
  words after the tool's name match against the leading words of a key, and the
  longest key within a source wins.
- A key that names a real command path, or starts with `help`, is shadowed: it
  never expands, and `aliases list` shows the problem. A key such as `books
  ls`, where `ls` is no subcommand of `books`, is allowed in a table.
- Expansion happens once per run, as ADR-0027 decided. Repeating to a depth
  with cycle detection stays rejected, because nothing asks for it.
- A source that cannot be read is skipped, and the later sources still apply.
  `expand` still never prints. `aliases list` shows one problem row for that
  source and exits with 1, so the `aliases::unreadable` code is removed.
- A file and a lazy source are read only when the first word is no command, as
  ADR-0027 decided for the file, so a run that types a command pays for no
  reading. A key of such a source with more than one word cannot start with a
  command, and `aliases list` says so. A table costs nothing to consult, so its
  keys can start with a command.
- The `Aliases:` line of the root's help names the entries of shipped sources,
  which are the same on every machine, unless the source is `.unlisted()`.
  Personal entries stay out unless the tool opts out of `Rule::PersonalAliases`,
  as before. crumbkit-test's checks drop personal sources, so golden files stay
  machine independent.
- `Aliases` replaces the free functions `expand`, `names` and `list`, and
  `Source::file` takes the path that `file` returns. `Help::personal_aliases`
  takes a function pointer, so a tool keeps its `Aliases` in a `static` and
  passes a small function that calls `Aliases::names`.

## Consequences

`docs/behaviour/parsing/aliases.md` describes the current behaviour. A tool
that used the free functions builds an `Aliases` once. ADR-0079 needs no change.

Considered: the longest key across all sources, repeated expansion to a depth
of 4, an opt-in for shipped entries in help, reading the file for every command
line so a personal key can start with a command.
