# ADR-0053. Recovery chains

- Date: 2026-10-13
- Status: accepted

## Context

A failure tells an agent what to run next. A recovery that leads to a
refusal, to the same failure again, or to nothing the agent can act on
wastes the agent's turns, and no check of a single failure can see that.

## Decision

- `check_recoveries` ran listed failing command lines and followed each
  `run` recovery and alternative they print, up to 3 steps by default. A
  step that the parser refuses fails the check, and so does a step that
  fails with a code its chain has already seen, because an agent would
  meet a failure it has already seen. A declared failure with no next
  step fails too, because an agent has nothing to act on.
- The runs of one chain shared fresh XDG directories, so a step sees what
  earlier runs wrote, such as a confirmation that a remove needs.
- A placeholder in a step took its value from the chain's own values,
  then from a field of the failure, then from shared values. A fill from a
  field is a finding, because an agent must copy that value itself, while
  a recovery built with `Call::value` carries it. A list field fills a
  step that takes every value, and that step counts as progress when it
  leaves fewer values than it was given.
- The check read a step's words as a POSIX shell does, so a tool need not
  re-quote its hints.
- The chains, codes and outcomes were pinned in `recoveries.txt` without
  messages, so a changed recovery shows in review.

## Consequences

Every declared code is provoked by a chain or listed with the reason no
test can provoke it. `docs/behaviour/checks/recovery-chains.md` describes
the current rules.

Considered: a field fill reported only in the golden file, an opt-out that names the step's command, a finding for a `Failing::values` entry that wins over a field, setup command lines that run before a start, a setup function that receives the XDG paths, shared values that win over a failure's fields, fields that match a placeholder's case exactly, a second read of the document's fields beside the failure, placeholders joined only by fixed separators, following recoveries from the failures that the examples provoke, generating failing command lines from the clap tree, a recovery check without a golden report, variables of a start that apply only to the start and not to its steps, a list or object field that fills a placeholder with its JSON text, a list field that fills a repeated placeholder such as `<ID>...` with one word for each item, an `each` step run once for every value of its field, a run with every value that fails with the same code counted as a finding even when it leaves fewer values.
