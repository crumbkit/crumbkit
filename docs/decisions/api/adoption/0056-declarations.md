# ADR-0056. Declarations

- Date: 2026-10-13
- Status: amended by ADR-0060

## Context

Tools that adopt crumbkit often already have users who read their failure
codes, exit codes, command names and completion variable. Treating each of
these as a departure that needs an opt-out with a reason made adoption
look like a list of exceptions, although the tool kept all of crumbkit's
behaviour.

## Decision

- crumbkit enforced behaviour and let a tool declare its own names.
  Behaviour is what an agent relies on across tools, such as when a
  failure happens, its recovery, the output budget, paging, confirmation
  and one meaning for each exit code. A name is a word the tool's users
  already read. A declaration needs no reason and no opt-out.
- A tool declared names through seams on the guide and leaves: code
  renames, exits that list crumbkit's codes, leaves that pass on a
  program's exit code, the completion variable's name, refusal
  alternatives and dashed forms. A leaf's refusal function replaces the
  guide's, so a refusal calls one function.
- Each check read through the seams, through `Guide::printed` and
  `Guide::failure_exit`, so a check expects what the run prints.
- `check_opt_outs` listed each declaration under the customisations, so a
  reviewer sees in one place how far a tool has moved from crumbkit's
  defaults. The completion variable is not listed, because the guide does
  not hold it.
- A new seam needs a name that the existing seams cannot declare, and a
  departure from behaviour stays an opt-out with a reason.

## Consequences

A tool that declares nothing prints crumbkit's codes and exits with
crumbkit's defaults. `docs/behaviour/adoption/declarations.md` lists the
current seams.

Considered: renames held in `Spelling`, an opt-out with a reason for a command named in another style, a trait that the tool implements to map each code and exit, an over-budget code on each `Document`, renames recorded as opt-outs with a reason, a fixed list of refusal alternatives on each leaf, a leaf's function that adds its calls to the guide's function's calls, the guide's function listed under each leaf that uses it, a `check_guide` rule that calls each function with a made-up refusal, the failures-in-order choice recorded as an opt-out with a reason.
