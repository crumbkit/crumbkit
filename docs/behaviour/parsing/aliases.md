# Aliases

This page covers the aliases of a tool's commands: the built-in aliases that
a tool declares in clap, and the aliases that come from sources, such as a
person's file or a table in the tool's code. The published crate
[crumbkit-aliases](../../../crates/crumbkit-aliases/README.md) holds the
sources, so only a tool with aliases from sources builds toml.

## Aliases are for people

A tool expands aliases before clap parses, so `commands list`, `Next:` lines,
recoveries and JSON print only real commands. Help prints only real commands
unless the tool opts in to showing aliases.

A built-in alias is a hidden clap alias, such as `ls` for `books list`, which
`check_names` accepts. A tool that wants a built-in alias in help makes it a
visible clap alias and opts out of `Rule::OneName` with a reason. Help then
prints it after the command's description as clap does, as in `[alias: ls]`, on
pages with goal headings too. Such an alias is the same on every machine and
costs only help bytes and a second spelling.

Why: ADR-0027.

## Sources

A tool builds one
[`Aliases`](../../../crates/crumbkit-aliases/src/aliases.rs) value from
[`Source`](../../../crates/crumbkit-aliases/src/source.rs)s in order. A source
is one of three kinds:

- `Source::file(path)` reads a person's `aliases.toml`. It is `Kind::Personal`.
- `Source::table(origin, entries)` holds entries in the tool's code. It is
  `Kind::Shipped` and costs no reading.
- `Source::lazy(origin, kind, read)` calls `read` for its entries. A tool
  uses it for entries that live somewhere that costs time to read.

A key is one or more words, such as `wt ls`, and its value is a list of one or
more words. The first source that has a matching entry wins, so a tool lists
the person's file first and its own sources after it. Within one source, the
longest key that the leading words spell wins. Why: ADR-0228.

## Expanding

The person's file is
[`file`](../../../crates/crumbkit-aliases/src/file.rs) in the tool's config
directory, which the etcetera crate finds by the XDG base directory rules. An
absolute `XDG_CONFIG_HOME` is the config directory without asking etcetera,
because etcetera first looks up the home directory, which reads the password
database when `HOME` is unset. The usage log finds its state directory the
same way ([usage log](../usage-log/README.md)).

A tool passes its arguments through
[`Aliases::expand`](../../../crates/crumbkit-aliases/src/expand.rs) before
`try_parse_to`, and passes `try_parse_to` the `Command` that `expand` read.
`expand` matches the words after the tool's name against the keys, replaces
the matched words with the entry's words once, with no shell, and appends the
words that followed. The replacement is not matched again (ADR-0027). A real
command always wins: a key that names a real command path, such as `books
list` or `books ls`, never expands. `expand` never prints or fails, and it
tells the usage log that an alias expanded.

When the first word is a command, `expand` leaves a lazy source unread, so a
run that types a command pays for no loading. A key of a lazy source that has
more than one word therefore cannot start with a command. A key of a file or a
table can, as in `books recent`, when the key as a whole is no command.

An entry that an earlier source wins over is never used. `aliases list` gives
it a problem that names the earlier source, and `names` leaves it out.

A source that cannot be read or parsed is skipped, and the later sources still
apply.

The hidden `<tool> aliases list` command
([`Aliases::list`](../../../crates/crumbkit-aliases/src/list.rs)) shows the
built-in aliases and the entries of every source, with the source of each row
in its `from` column and the problem of each entry that does not expand. A
source that cannot be read has one row with the reason as its problem. The
command exits with 1 while a row has a problem
([rows that report a problem](../output/listings/README.md#rows-that-report-a-problem)).

Why: ADR-0027, ADR-0029, ADR-0050, ADR-0228.

## Aliases in help

A tool that wants aliases in help sets `Help::aliases` to a function of its
own that takes the clap root and a `Scope`. It returns `Aliases::shipped_names`
for `Scope::Shipped` and `Aliases::names` for `Scope::All`. The clap root's
`-h` and `--help` then end with a line such as `Aliases: recent, wt ls.` after
the `Topics:` line. `parse` calls the function only when it prints that help,
so no other run reads a source. `Aliases::names` leaves out the entries of a
source added with `Source::unlisted`. `Aliases::shipped_names` judges only the
sources of `Kind::Shipped`, so it never reads a person's file, and its names
are the same on every machine.

The scope is `Scope::Shipped` unless the tool sets `Help::alias_scope` to
`Scope::All`. A tool that sets `Scope::All` shows the entries of a person's
file, which differ between machines, so it opts out of `Rule::PersonalAliases`,
which `check_names` enforces, and the opt-out report says that help, golden
files and what an agent sees then differ ([opt-outs](../adoption/opt-outs.md)).
shelf opts in to show the line.

crumbkit-test's help checks print help with `Scope::Shipped`, whatever scope
the tool sets. `check_examples` and
`cargo crumbkit weigh` run the binary with an empty config directory, so
golden files and budgets are the same on every machine.

Why: ADR-0027, ADR-0228.
