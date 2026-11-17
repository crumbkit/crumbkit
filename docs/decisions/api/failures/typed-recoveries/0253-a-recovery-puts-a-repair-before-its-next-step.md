# ADR-0253. A recovery puts a repair before its next step

- Date: 2026-11-17
- Status: accepted
- Supersedes: ADR-0001, which it amends

## Context

Many failures say to make a repair and then run the command again, or then run
another command. `Recovery::Retry` and `Recovery::Run` cannot carry the repair,
so these stayed free-text hints that `check_recoveries` does not parse.
`Recovery::Retry` also did not say what a retry promises, so a hint that says
when a retry passes stayed free text.

## Decision

- `Recovery` gains the variant `After { repair, then }` and the constructor
  `Recovery::after(repair, then)`. `then` is a `Then`, a public type exported
  at the crate root, with the variants `Run(Call)`, `Terminal(Call)` and
  `Retry`. The type admits only a step that can follow a repair. `Then` is
  `#[non_exhaustive]`, which leaves room for a later variant such as
  `Program`.
- The `help:` line prints the repair, `, then` and the step: ``move the file
  away, then run the same command again`` or ``move the file away, then run
  `tool books list` ``. A `then` call that does not render makes the whole
  recovery print as `None` with crumbkit's reason, and a debug build warns. A
  `Then::Terminal` call with a placeholder prints the same way.
- The failure document gains a step kind `after` with the required properties
  `repair` and `then`. `then` is a `run`, `terminal` or `retry` step in the
  shape each has alone. The kind is valid for `recovery` and for each entry of
  `alternatives`. The id stays `crumbkit/failure/v1`, as the second step kind
  that breaks `v1` in place under the exception ADR-0252 records.
- The reader type `RecoveryStep` gains `After { repair, then }`. The recovery
  check reads a `then` of kind `run`, `terminal` or `retry` and reports any
  other as a step it cannot read.
- `check_recoveries` follows the step after the repair. A `retry` ends the
  branch, and the report says `after a repair, retry`. A call runs as a `run`
  step, and the report says `after a repair, run <call>`. A call that the
  parser refuses is a problem. A call that fails with the code of the failure
  that offered the step is no problem, because the check cannot make the
  repair; the report says `fails with <code>, as before the repair` and the
  branch ends. Any other code continues the chain. An `after` step counts as a
  next step.
- `Recovery::Retry` carries a contract. A tool offers it only when the next
  run starts clean, because the command removes or reuses what a failed run
  left, such as a partial file or a stale lock. When the caller must repair
  something first, the tool offers `Recovery::after` with `Then::Retry`. No
  check measures the contract, so a tool that offers `Retry` today is not
  re-checked.

## Consequences

No document written before this change holds an `after` step, so each stays
valid. A reader that refuses an unknown step kind refuses a document that
holds one, and a tool is affected only after it upgrades crumbkit and adopts
`Recovery::after`. Every pinned schema that embeds the failure schema changes
at an adopter's next bump and is regenerated there.

Considered: `then: Box<Recovery>`, which admits a repair before a hint, a none
or another repair, a `hint` step with optional typed fields, a `failure/v2`
id, a derive key such as `repair = ".."`, and a check that a tool's `Retry`
keeps its contract.
