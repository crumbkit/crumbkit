# Aliases

This page covers the aliases of a tool's commands: the built-in aliases that
a tool declares in clap, and the personal aliases that a person writes in a
file. The published crate
[crumbkit-aliases](../../../crates/crumbkit-aliases/README.md) holds the
personal aliases, so only a tool with personal aliases builds toml.

## Aliases are for people

A tool expands aliases before clap parses, so `commands list`, `Next:` lines,
recoveries and JSON print only real commands. Help prints only real commands
unless the tool opts in to showing aliases.

A built-in alias is a hidden clap alias, such as `ls` for `books list`, which
`check_names` accepts. A tool that wants a built-in alias in help makes it a
visible clap alias and opts out of `Rule::OneName` with a reason. Help then
prints it after the command's description as clap does, as in `[alias: ls]`,
on guided pages too. Such an alias is the same on every machine and costs
only help bytes and a second spelling.

Why: ADR-0027.

## Personal aliases

A personal alias is an entry of `aliases.toml` in the tool's config
directory, which the etcetera crate finds by the XDG base directory rules
([`file`](../../../crates/crumbkit-aliases/src/file.rs)). An absolute
`XDG_CONFIG_HOME` is the config directory without asking etcetera, because
etcetera first looks up the home directory, which reads the password
database when `HOME` is unset. The usage log finds its state directory the
same way ([usage log](../usage-log/README.md)).

A tool passes its arguments through
[`expand`](../../../crates/crumbkit-aliases/src/expand.rs) before
`parse_from`. `expand` replaces only the first word, once, with no shell, and
a real command always wins. It never prints or fails, and it tells the usage
log that an alias expanded.

The hidden `<tool> aliases list` command
([`list`](../../../crates/crumbkit-aliases/src/list.rs)) shows the built-in
aliases and the file's entries, with the problem of each entry that does not
expand. It exits with 1 while an entry has a problem
([rows that report a problem](../output/listings/README.md#rows-that-report-a-problem)).

Why: ADR-0027, ADR-0029, ADR-0050.

## Personal aliases in help

A tool that wants a person's aliases in help sets `Guide::personal_aliases`
to [`names`](../../../crates/crumbkit-aliases/src/names.rs) and opts out of
`Rule::PersonalAliases`, which `check_names` enforces. The clap root's `-h`
and `--help` then name each entry that expands on one line, after the
`Topics:` line. `parse` calls `names` only when it prints that help, so no
other run reads the file. Those names differ between machines, so the
opt-out report says that help, golden files and what an agent sees then
differ ([opt-outs](../adoption/opt-outs.md)). shelf opts in to show the line.

crumbkit-test's help checks print help without a person's aliases, through
`Guide::without_personal_aliases`. `check_examples` and
`cargo crumbkit weigh` run the binary with an empty config directory, so
golden files and budgets are the same on every machine.

Why: ADR-0027.
