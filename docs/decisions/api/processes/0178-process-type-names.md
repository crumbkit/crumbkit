# ADR-0178. The process crate's types have names of their own

- Date: 2026-11-08
- Status: accepted
- Supersedes: ADR-0172 and ADR-0174, which it amends
- Missed: ADR-0172 and ADR-0174 used `Finished` and `Exit` without considering that crumbkit's `events::Finished` and `help::Exit` mean other things to a tool that imports both crates.

## Context

crumbkit has `events::Finished`, the event at the end of a tool's own run,
and `help::Exit`, a declared exit code. crumbkit-process had `Finished`,
the result of one child, and `Exit`, how that child ended, and its `Run`
held the settings of a run. A tool that imports both crates had to alias
one of each pair, and the compiler cannot warn about the clash, because
ADR-0171 keeps crumbkit-process free of crumbkit.

## Decision

- `Run` is `Limits`, because it holds what a run reads, keeps and allows.
- `Exit` is `Outcome`.
- `Finished` is `ChildFinished`.
- The `run` function, `RunError` and `Outcome`'s variants, including
  `Outcome::Stopped`, keep their names. `ChildFinished::exit` keeps its name.

## Consequences

A tool that names any of the three types changes the name at 0.2. ADR-0002
allows a breaking change in a 0.x minor release.

Considered: keeping the names and leaving each tool to alias its imports,
which leaves one name for two concepts in any tool that uses both crates.
