# ADR-0076. parse returns Output

- Date: 2026-10-17
- Status: accepted
- Supersedes: ADR-0015 and ADR-0060, which it amends
- Missed: ADR-0060 gave a tool that parses with clap alone only the guide's renames, because `Parsed::guided` had no typed words with which to find the leaf.

## Context

A tool flattened `Output` into its clap types, and its `main` passed the
`Parsed` that `parse` returned to `Output::set_command`. A tool that forgot
the call printed no `Next:` line and no `Run` recovery, and a debug build
warned about it. `commands list`, `schemas list` and `aliases list` each
had a failure code for a run without a command. A tool that parsed with
clap alone built its `Parsed` with `Parsed::guided`, which had no typed
words, so it got the guide's code renames and never the renames of the
leaf that the run named.

## Decision

- `parse`, `parse_from` and `parse_to` return the tool's parsed type and an
  `Output` that already holds the command tree, the guide's settings, the
  typed words and the renames of the leaf that the words name.
- A tool flattens `ViewFlags` in place of `Output`. `ViewFlags` holds
  `--json`, `--fields`, `--long` and `--full`, and `parse` reads them
  before the tool's type takes its values from clap's matches. A tool that
  does not flatten `ViewFlags` parses with every view flag off.
- A tool that parses with clap alone builds its `Output` with
  `Output::new(view, command, guide, args)`. `args` is the argv with the
  program name first. `Output::new` finds the leaf with `command_path`, so
  the run gets the leaf's code renames, and it keeps the words for the
  `--yes` recovery of `Output::confirm`.
- `Output::set_command`, `Parsed`, `Parsed::guided` and the debug-build
  warning about a missing command are removed. `Output` no longer derives
  `clap::Args` or `Default`, and `Output::command` returns `&Command`.
- The codes `crumbkit::commands::unavailable`,
  `crumbkit::schemas::unavailable` and `crumbkit::aliases::unavailable` are
  removed, because every `Output` holds a command.
- The rest of ADR-0015 and ADR-0060 is unchanged.

## Consequences

A tool's `main` cannot forget to hand crumbkit its command tree, and a
handler takes the `Output` from the result of `parse`. A tool that parses
with clap alone prints the same codes as one that calls `parse`. Every
tool that flattened `Output` changes one field and its `main`, which is a
breaking change.

Considered: keeping `set_command` beside the new return value; an
`Output::new` without the argv, which would leave a tool that parses with
clap alone without its leaf's renames; keeping `Output` flattenable with
an `Option<Command>` inside it.
