# ADR-0027. Aliases

- Aliases are for people. A tool expands them before clap parses, so help, `commands list`, `Next:` lines, recoveries and JSON print only real commands.
- A built-in alias is a hidden clap alias, such as `ls` for `books list`, which `check_names` accepts.
- A personal alias is an entry of `aliases.toml` in the tool's directory under the config directory that etcetera's base strategy gives, such as `mine = ["books", "list"]`. That is `$XDG_CONFIG_HOME/<tool>/aliases.toml`, or `~/.config/<tool>/aliases.toml` when that variable is unset, empty or relative, and etcetera reads the home directory from `HOME`, or from the password database when `HOME` is unset or empty. The usage log finds its state directory the same way (ADR-0029).
- `expand` replaces only the first word, once, with no shell. A first word that is `help`, a root subcommand or its alias wins and skips the file read, as does a flag.
- `expand` never prints or fails, and leaves the arguments unchanged for a bad file or entry. When it replaces an alias, it calls `crumbkit::usage::aliased()`, so the usage log records the run as an alias. The hidden `<tool> aliases list` reports the problem of each entry, and its text note names the file.
- Its rows have `name`, `command`, `problem` and `from`: built-in aliases first by full path, such as `books ls`, then entries by name. The document id is `crumbkit/aliases/v1`, and a file that cannot be read or parsed fails with `crumbkit::aliases::unreadable`.
- The published crate crumbkit-aliases holds this, so only a tool with personal aliases builds toml.

Considered: the `dirs` crate, reading `XDG_CONFIG_HOME` and `HOME` by hand, a warning on stderr, aliases anywhere in the command line, recursive or shell aliases, built-in alias names without their path.
