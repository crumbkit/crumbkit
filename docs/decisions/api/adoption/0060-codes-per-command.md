# ADR-0060. Codes per command

- Date: 2026-10-14
- Status: accepted
- Supersedes: ADR-0056, which it amends

## Context

A tool's renames of crumbkit's codes applied to every run of the tool. Some
tools already print a different code for one of crumbkit's failures in one
command than in the rest, such as a version conflict that one command
reports as a moved row and every other command reports as a stale row.
Such a tool could rename the code for every command or for none, so its
users would see a code change in the commands that it did not fit.

## Decision

- A leaf could list renames with `Leaf::codes`. A run of the leaf's command
  read the leaf's renames first and the guide's renames for every other
  code. `parse` found the leaf from the parsed command path and passed its
  renames to `Output` through `Parsed`.
- A leaf could not rename `crumbkit::cli::usage`. crumbkit refuses a command
  line before it knows the command, so a refusal could not read a leaf. The
  rule was enforced when a `const` guide compiled, so `check_exit_codes`
  needed no problem for it.
- `Guide::printed` and `Guide::failure_exit` took the command path, so each
  check asked for the code and the exit of the run that it made. A path that
  named no leaf with renames gave the guide's renames.
- `Parsed::guided` read only the guide's renames, because it has no typed
  words. `RuntimeLeaf` had no renames, because no tool asked for them.
- `check_exit_codes` read every list of renames and every leaf's view of
  the exits, and `check_opt_outs` listed a leaf's renames as `code-rename`
  customisations of the leaf's command.

## Consequences

A tool can keep its own code for one of crumbkit's failures in one command
and still list that code in an exit. The usage log and the trace record the
leaf's code. The command tree is not cloned, because `Parsed` and `Output`
hold the leaf's renames as a borrowed slice.

Considered: a rename keyed by a command path on the guide, renames on `RuntimeLeaf`, a leaf rename of the usage code that only commands with no refusal read, a `check_exit_codes` problem in place of the compile-time rule, a new `Guide::printed_at` beside an unchanged `Guide::printed`.
