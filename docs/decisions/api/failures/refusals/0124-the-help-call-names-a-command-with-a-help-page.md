# ADR-0124. A refusal's help call names a command with a help page

- Date: 2026-10-20
- Status: accepted
- Supersedes: ADR-0001 and ADR-0078, which it amends
- Missed: ADR-0001 and ADR-0078 named the `-h` of the deepest typed command and missed that a command can turn its help flag off, so the recovery could name a call that clap refuses.

## Context

A refused command line recovered with the `-h` of the deepest typed
command, even a hidden one. When the tool's function supplied the
recovery, the same call became the first alternative. An adopting tool has
a hidden leaf that answers a shell's completion requests. The leaf turns
its help and version flags off and takes every word after `--`. A refusal
of a command line that reaches the leaf, such as one that types `--help`
before the `--`, printed a help line that runs the leaf's `-h`. clap
refuses that call with "unexpected argument '-h' found", so an agent that
followed the recovery met a second refusal.

## Decision

- The `-h` call of a refusal names the deepest command on the typed path
  that has a help flag, from the deepest typed command up to the root. A
  hidden command with a help flag still counts.
- A command has a help flag when neither it nor a command above it turned
  clap's flag off with `disable_help_flag`, which clap applies to every
  command below the one that sets it. A command also has one when it
  declares its own `-h` argument that prints help, or a command above it
  declares one as a global argument.
- When no command on the path has a help flag, the recovery is
  `Recovery::None` with a reason that says no command on the command line
  has a `-h` help page. When the tool's function supplies the recovery,
  the refusal then offers no `-h` alternative.
- One function chooses the call for the recovery and for the alternative
  from the command tree and the path, so the two always agree.
- crumbkit added no option. The rule corrects a recovery that named a call
  that fails.

## Consequences

`docs/behaviour/failures/README.md` and `docs/behaviour/parsing/README.md`
describe the rule. A refusal at a leaf without a help flag points the
caller at the help of the group above it, whose list leaves out a hidden
leaf. A tool whose root turns its help flag off and declares no `-h` of its
own prints refusals that offer no step. crumbkit-test's `check_recoveries`
needed no change: it runs a `-h` step as any other run, and a `none` step
on crumbkit's usage code is no dead end.

Considered: keeping the deepest command and documenting that a tool which
turns its help flag off gets a call that fails; a `--help` call for a
command that declares `--help` without `-h`; a hint that names the leaf's
usage in place of the help call; a `Recovery::None` at the deepest command
without walking up to a command that has a help page.
