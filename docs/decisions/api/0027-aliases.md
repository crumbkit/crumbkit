# ADR-0027. Aliases

- Date: 2026-10-10
- Status: accepted

## Context

People like short names for the commands they type often, but an agent
reads a tool's commands from its help, `commands list` and JSON, and
those must be the same on every machine. Golden files and help budgets
must also not depend on one person's configuration.

## Decision

- A tool expanded aliases before clap parses, so every output that an
  agent reads names only real commands.
- A built-in alias was a hidden clap alias. A tool that wants one in help
  makes it visible and opts out of `Rule::OneName` with a reason, because
  such an alias is the same on every machine and costs only help bytes.
- A person's aliases lived in `aliases.toml` in the tool's config
  directory. The root help names them only when the tool opts out of
  `Rule::PersonalAliases`, because the line differs between machines, and
  only that help reads the file.
- crumbkit-test's checks and `weigh` ran without a person's aliases, so
  golden files and budgets are the same on every machine.
- `expand` replaced only the first word, once, with no shell, and a real
  command always wins, so an alias cannot hide or redefine a command. It
  never prints or fails, so a broken file does not break the tool. The
  hidden `aliases list` command shows each entry's problem instead.
- An absolute `XDG_CONFIG_HOME` was used without asking etcetera, because
  etcetera looks up the home directory first, which reads the password
  database when `HOME` is unset.
- The separate crumbkit-aliases crate held this, so only a tool with
  personal aliases builds toml.

## Consequences

The usage log records that an alias expanded. `docs/behaviour/parsing/aliases.md`
describes the current behaviour.

Considered: one opt-in for built-in and personal aliases, a crumbkit setting that prints hidden clap aliases, a new rule for visible command aliases, crumbkit-aliases writing the line into the root's `after_help` before parsing, a fixed line that names no alias, a general hook for a line of the root's help, a section that lists each personal alias with its command, the `dirs` crate, reading `XDG_CONFIG_HOME` and `HOME` by hand, a warning on stderr, aliases anywhere in the command line, recursive or shell aliases, built-in alias names without their path.
