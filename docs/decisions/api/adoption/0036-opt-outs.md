# ADR-0036. Opt-outs

- Date: 2026-10-12
- Status: amended by ADR-0061

## Context

crumbkit's checks enforce conventions that an existing tool may not meet
on day one, and some tools have a good reason to depart from one rule for
one command. A blanket exemption would hide every other problem of that
command, and a forgotten exemption would outlive the reason for it.

## Decision

- A tool opted out of one rule for one command and the commands below
  it, with a reason, through an `Allow`. The other rules of the check still
  apply, and a tool keeps one list that it passes to every check.
- A check failed on an opt-out that names no command or excuses no
  problem, so the opt-out of a removed or fixed command cannot stay in a
  tool's list.
- Some rules got no opt-out, each for its own reason. `help` stays
  reserved because `parse` answers it itself, crumbkit's commands stay at
  fixed paths, failure codes stay unique, and the field-name rule stays
  because the failure document's schema promises it to every reader.
- `check_opt_outs` pinned the opt-outs, budgets and departing guide
  settings in a text and a JSON golden file, with what each costs an
  agent, so a reviewer sees a new opt-out in review and an agent can read
  where the tool departs from crumbkit's conventions. Declarations are
  listed after them under the customisations, with no reason and no cost.
  The report is a test check only, because a tool's opt-outs live in its
  tests.

## Consequences

A tool can turn the checks on one at a time and add the report when it
wants its opt-outs pinned. `docs/behaviour/adoption/opt-outs.md` lists
the current rules that take opt-outs.

Considered: exemptions as bare command paths, an exemption that skips every rule of a check, one constructor per rule in place of a public `Rule` enum, an opt-out that covers its command alone, an opt-out from the reserved word `help`, a `cargo crumbkit` command that prints the report, declarations left out of the report, declarations listed among the opt-outs with a reason.
