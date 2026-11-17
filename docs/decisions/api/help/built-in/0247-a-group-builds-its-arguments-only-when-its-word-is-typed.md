# ADR-0247. A group builds its arguments only when its word is typed

- Date: 2026-11-17
- Status: accepted

## Context

`Cli::command()` builds every command and argument of every variant of a
tool's enum on each run, and the run then drops them. A profile of
`shelf -h` put about 120,000 of the 149,222 instructions added since 0.4.0
in clap building and dropping the argument trees of the built-in groups
`self runs`, `completion`, `skills` and `self`. The derive gives a tool no
way to skip a group's arguments.

## Decision

- `crumbkit::Lazy<S>` is a newtype for a `#[command(subcommand)]` field. Its
  `Subcommand::augment_subcommands` calls clap's stable `Command::defer`, so
  the group's subcommands are added when clap builds the group. A tool
  writes `Lazy<S>` where it wrote `S` and reads the value with `into_inner`.
- `Lazy` sets the subcommand value name to clap's default, `COMMAND`, when
  the tool set none. Help prints the same, and crumbkit recognises a group
  that has no subcommands yet by the name and by having no subcommands.
  clap offers no getter for a deferred function.
- A deferred function runs after the variant's own attributes, where clap's
  derive applies them before. `Lazy` therefore keeps the about, help and
  version texts that the variant gave, so a variant's doc comment still
  wins over the enum's.
- `crumbkit::reach(command, words)` expands each lazy group that the words
  pass through. It builds each command from the root down to the group, the
  group last. clap copies a command's global arguments only into its direct
  subcommands when it builds that command, and builds no command twice, so a
  group built before its parent would miss the root's global flags such as
  `--json`. It sets `disable_help_subcommand(true)` on the root first, and it
  hides a command's visible global flags from `-h` before it builds the
  command, as the help lines of `parse` do for the commands on the path.
- `parse` calls `reach` before any reader of the tree, and again with the
  words after `help`. `Aliases::expand` takes `&mut Command` and calls it
  with the typed words. `help::extend` and crumbkit-test's checks expand every lazy group
  in a tree first. A run that types no lazy group's word builds nothing.

## Consequences

The change breaks crumbkit-skills, crumbkit-usage and completion mounts
only in that a tool writes `Lazy<..>` for the groups it wants lazy. A tool's
own code that reads `Cli::command()` without building it sees a lazy group
without subcommands.

Considered: a macro per group that writes the enum and its field, which
makes a tool learn six macros and does not help a tool's own groups; a change
to clap that skips an unused group's build, which crumbkit cannot make;
building the whole tree in `reach`, which costs what the change removes;
expanding a group when its word is read by each tree reader, which repeats the
root-first rule in every reader.
