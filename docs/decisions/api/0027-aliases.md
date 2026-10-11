# ADR-0027. Aliases

- Aliases are for people. A tool expands them before clap parses, so help, `commands list`, `Next:` lines, recoveries and JSON print only real commands.
- A built-in alias is a hidden clap alias, such as `ls` for `books list`, which `check_names` accepts.
- A personal alias is an entry of `aliases.toml` in the tool's config directory, which the etcetera crate finds by the XDG base directory rules ([`file`](../../../crates/crumbkit-aliases/src/file.rs)). The usage log finds its state directory the same way (ADR-0029).
- [`expand`](../../../crates/crumbkit-aliases/src/expand.rs) replaces only the first word, once, with no shell, and a real command always wins. It never prints or fails, and it tells the usage log that an alias expanded.
- The hidden `<tool> aliases list` command ([`list`](../../../crates/crumbkit-aliases/src/list.rs)) shows the built-in aliases and the file's entries, with the problem of each entry that does not expand.
- The published crate crumbkit-aliases holds this, so only a tool with personal aliases builds toml.

Considered: the `dirs` crate, reading `XDG_CONFIG_HOME` and `HOME` by hand, a warning on stderr, aliases anywhere in the command line, recursive or shell aliases, built-in alias names without their path.
