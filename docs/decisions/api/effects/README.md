# ADR-0038. Effects

- Date: 2026-10-12
- Status: accepted

## Decision

These records decided how a command tells a caller what it changes: the
effects that each leaf declares, dry runs, the confirmation that a
destructive command asks for, the questions that pick a missing
argument's values, with how crumbkit decides that a person can answer,
and the guards of a write that replaces data or writes many items.

## Children

- [ADR-0039. Effects, dry runs and confirmation](0039-effects-dry-runs-and-confirmation.md)
- [ADR-0043. Pickers and interactive input](0043-pickers-and-interactive-input.md)
- [ADR-0047. Write guards](0047-write-guards.md)
