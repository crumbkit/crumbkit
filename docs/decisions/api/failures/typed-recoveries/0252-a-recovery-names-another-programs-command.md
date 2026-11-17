# ADR-0252. A recovery names another program's command

- Date: 2026-11-17
- Status: accepted
- Supersedes: ADR-0001 and ADR-0007, which it amends

## Context

A failure that tells the caller to run another tool's status or help command
could only say so in a `Recovery::Hint`. `Program` and `Step::run` exist only
on listings, so the command line was free text that nothing quoted and that
`check_recoveries` could not read. A program that reads the failure document
could not tell such a command from advice.

ADR-0007 says that adding an enum value needs a new `/vN` id.

## Decision

- `Recovery` gains the variant `Program { program, args }` and the constructor
  `Recovery::program(program, args)`, which mirror `Step::run`.
- The `help:` line prints ``run `git status` ``, with each argument quoted as
  `Step::run` quotes it. An argument with a control or format character makes
  the recovery print as `None` with the reason that the suggested command is
  not available, and a debug build warns.
- The failure document gains a step kind `program` with the required
  properties `program`, `args` and `run`, where `run` is the quoted command
  line that the text prints. The kind is valid for `recovery` and for each
  entry of `alternatives`. The earlier kinds do not change.
- The id stays `crumbkit/failure/v1`. This is an exception to ADR-0007: while
  crumbkit's own tools are the only users of `failure/v1`, a new step kind
  goes into `v1`, and the CHANGELOG marks the change as breaking.
- `check_recoveries` writes `program` and the command line in its report,
  ends the branch without running the program, and counts the step as a next
  step.

## Consequences

No document written before this change holds a `program` step, so each stays
valid. A reader that refuses an unknown step kind refuses a document that
holds one, and a tool is affected only after it upgrades crumbkit and adopts
`Recovery::program`. Every pinned schema that embeds the failure schema
changes at an adopter's next bump and is regenerated there.

Considered: a `hint` step with optional typed fields, which leaves advice and
a command in one kind that a reader must tell apart by its fields, and a
`failure/v2` id, which makes every reader of `v1` migrate for a change that
no existing document shows.
