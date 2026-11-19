# ADR-0259. cargo crumbkit's skill suggests guides from a tool's usage

- Date: 2026-11-19
- Status: accepted

## Context

A tool's `self usage` reports where callers waste effort, such as a help
loop, a usage error, an identical repeat or heavy output (ADR-0034). In
two weeks of one tool's log, 1,240 of 1,566 runs came from a coding agent,
and one command was repeated 76 times within a minute of itself. Nothing
turned those findings into a change. An author read the report, guessed
why agents did each thing, and decided whether the fix was a line in the
tool's skill or a change to the tool. `tool-skill draft --usage` ranks the
most-used commands for a skill (ADR-0194), and it says nothing about wasted
effort.

## Decision

- cargo crumbkit's skill gains a second job, "Suggest guides from usage",
  beside "Audit a tool" (ADR-0199). One run reads one named tool's
  `self usage --json --full`, `self runs list --all --json` with
  `--fields` naming the long fields it reads, such as `flags` and
  `caller`, `self runs export --json` and `commands list --json --full`,
  for a window that defaults to 14 days.
- `GUIDES.md` ships beside the skill with one row for each finding of
  `self usage`. A row says what agents were probably trying to do, a
  template for one line of guide text, the tool change that would remove
  the cause, the affordances from `AFFORDANCES.md` that the change would
  use, and when the guide text alone is enough.
- The job judges a finding only when it has at least 5 runs, because a
  pattern seen fewer times gives too little evidence of what agents
  wanted, and lists the others under `not_judged`.
- The job checks every guide line with `tool-skill check` before it
  writes it, so no suggestion names a command or flag the tool lacks.
- The findings of one command become one suggestion, because they usually
  share a cause and one task per cause is enough.
- The job finds the runs that a program made, such as a poller, and gives
  them a tool change with no guide text, because no agent reads it. It
  reads the runs that a program marked with `<TOOL>_PROGRAM` (ADR-0197)
  first. For unmarked runs it judges by flags and timing, because an
  unmarked program inherits the agent's environment and its `caller`
  names the agent.
- A cost test moves a finding that costs a caller nothing, such as a slow
  command under 5 seconds, to a `rejected` list, which never makes the
  outcome fail. A trial on one tool's log turned such findings into
  suggestions that no reader would act on.
- When the caller's instructions say that it keeps tasks in Board for the
  tool's repository, the job writes one task per suggestion. Board on
  PATH alone is not enough, because `tasks.add` writes to another
  repository when no board mounts the tool's. Otherwise it writes them to the place the caller's
  instructions name. In every case it prints the
  `cargo-crumbkit/guide-report/v1` report, whose schema ships beside the
  skill and follows the audit report's shape.
- A cause that the usage log cannot show is a proposal for crumbkit, as
  the audit's `crumbkit-gap` is.

## Consequences

The job adds no command to cargo crumbkit, so it uses only what the tool
already serves. A new finding in `self usage` needs a row in `GUIDES.md`,
and a test fails until the row exists. The job never reads the values a
caller typed, because the log holds none (ADR-0029).

Considered: a cargo crumbkit command that computes suggestions, which
cannot judge what an agent was trying to do; a separate skill, which an
author would install and find apart from the audit; reading every tool on
a machine in one run, which can come later; applying the suggestions,
which leaves the author no review; writing only to Board, which serves
only callers that use it.
