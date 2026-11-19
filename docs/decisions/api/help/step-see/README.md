# ADR-0258. A guide step points to another tool's topic

- Date: 2026-11-19
- Status: accepted

## Context

A task guide often needs a step that another tool carries out, such as signing
in with a separate login tool. Before this decision a guide could name that
step only in prose, which no check read. A tool must not link to another
tool's code to learn its commands, so `check_help` cannot parse them.

## Decision

- `Step::see(about, tool, topic)` declares a step with an explanation and a
  pointer to the topic `topic` of the tool `tool`, both `'static` names.
  `Step::pointer` returns them, and `Step::commands` is empty.
- `<tool> help <guide>` prints the explanation and the line
  `$ <tool> help <topic>` under it.
- `check_help` fails a `tool` that is not a command name in the form of a
  topic name, a `topic` that is not a topic name, and a `tool` equal to the
  tool's own name, because a step of the same tool needs a step with
  commands. A pointer step is exempt from the rule that a step has a command.
- The step counts toward `Topic::MAX_STEPS`, and `Step::MAX_COMMANDS` does
  not apply to it.
- This relaxes ADR-0257's rule that each step has at least one command: a
  step made by `Step::see` has none.
- `check_help` does not run or parse the other tool's commands, and it does
  not check that the other tool has the topic. The other tool's own
  `check_help` keeps its topic's commands true.

API-1 ground: an ordinary need that other tools share, stated by the same
tools that asked for the guide topic.

## Consequences

When the other tool removes the topic, no test in this tool fails, and a
reader gets the other tool's unknown-topic refusal. That is by design,
because the check holds no other tool's command tree.

Considered: pointing to a leaf or a command of another tool, which needs that
tool's command tree to check; resolving or fetching the other tool, which
needs state that a check does not have; cross-tool version pinning.
