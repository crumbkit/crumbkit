# ADR-0027. Aliases

- Aliases are for people. A tool expands them before clap parses, so help, `commands list`, `Next:` lines, recoveries and JSON print only real commands.
- A built-in alias is a hidden clap alias, such as `ls` for `books list`, which `check_names` accepts.
- A personal alias is an entry of `$XDG_CONFIG_HOME/<tool>/aliases.toml`, or `$HOME/.config/<tool>/aliases.toml` when that variable is unset, empty or relative, such as `mine = ["books", "list"]`.
- `expand` replaces only the first word, once, with no shell. A first word that is `help`, a root subcommand or its alias wins and skips the file read, as does a flag.
- `expand` never prints or fails, and leaves the arguments unchanged for a bad file or entry. The hidden `<tool> aliases list` reports the problem of each entry.
- Its rows have `name`, `command`, `problem` and `from`: built-in aliases first by full path, such as `books ls`, then entries by name. The document id is `crumbkit/aliases/v1`, and a file that cannot be read or parsed fails with `crumbkit::aliases::unreadable`.
- The published crate crumbkit-aliases holds this, so only a tool with personal aliases builds toml.

Considered: the `dirs` crate, a warning on stderr, aliases anywhere in the command line, recursive or shell aliases, built-in alias names without their path.
