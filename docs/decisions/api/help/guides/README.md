# ADR-0257. Guide topics

- Date: 2026-11-19
- Status: accepted

## Context

A topic held plain text only, so crumbkit could not tell a command line in it
from prose. `check_help` parsed the examples of leaves and never a command in
a topic, so a guide that told a reader to run `records create --title x` kept
passing after the tool renamed `--title`. Several tools built on crumbkit want
a multi-step task guide in their own help.

## Decision

- `Topic::guide(name, about, steps)` declares a guide. `Step::new(about,
  commands)` holds a one-line explanation and one or more commands, each the
  words after the tool's name.
- `<tool> help <name>` prints one numbered line for each step and each of its
  commands on an indented line that starts with `$`, with the words quoted as
  `help::quote` quotes example lines.
- `check_help` parses each command with the tool's name in front and fails
  naming the topic, the step number, clap's error kind and the command line.
  A `--help` or `--version` command counts as refused.
- A guide has at most 12 steps, a step at most 3 commands, and each step has
  at least one command and an explanation of one line. `check_help` fails a
  guide that breaks a limit, and the limits are `Topic::MAX_STEPS` and
  `Step::MAX_COMMANDS`.
- `Topic::text` returns the text of a plain topic and an empty string for a
  guide, because a guide's text holds the tool's name, which a `Topic` does
  not know. `Topic::render(tool_name)` returns what help prints for both
  kinds, and `Topic::steps` returns a guide's steps.

## Consequences

A plain `Topic::new` keeps its meaning. A tool that reads topics itself and
prints `text()` shows nothing for a guide and calls `render` to show one.
The root's `Topics:` line names guides with the other topics.

Considered: parsing command lines out of topic text, which needs a shell
grammar and guesses where a command starts; a guide step that points to
another tool's topic, which a later change adds; running the commands, which
needs state that a check does not have.
