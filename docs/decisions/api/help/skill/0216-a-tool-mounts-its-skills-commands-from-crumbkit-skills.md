# ADR-0216. A tool mounts its skills commands from crumbkit-skills

- Date: 2026-11-13
- Status: accepted
- Supersedes: ADR-0192, which it amends
- Missed: ADR-0192 chose three functions that each tool mounts in its own enum, and did not weigh that every tool would copy the same enum, leaves and dispatch.

## Context

crumbkit-skills exported `install`, `check` and `print` with their argument types and documents. shelf, cargo-crumbkit, multiverse and deadband each wrote the same three parts by hand: three `Leaf` records with one set of examples, effects and documents, a `SkillsCommand` enum, and a match that wraps each result in `Ok` and passes it to `finish` with its document. A tool that adopts crumbkit-skills copies them again.

## Decision

- crumbkit-skills exports `SkillsCommand`, a `#[non_exhaustive]` clap `Subcommand` with `Install(SkillsInstall)`, `Check(SkillsCheck)` and `Print(SkillsPrint)`. `SkillsPrint` has no arguments and is `#[non_exhaustive]`, so a name argument can be added later without a breaking change. The about lines say "this tool's skills", because the enum is shared.
- `run(run, skills, &command)` runs the command and passes the result to `finish` with its document. `run_to(sink, run, skills, &command)` does the same through `finish_to`, as crumbkit exports both, for a tool that tests against a `Sink`.
- `install_leaf`, `check_leaf` and `print_leaf` are `const fn`s that take the leaf's path, because a `const fn` cannot join a prefix to a path. A tool that mounts `skills` under another group, such as cargo-crumbkit under `crumbkit`, gives its own path, and chains `.examples(..)` for examples of its own.
- `skills print` prints `skills[0]`, so a tool lists its main skill first. When the slice is empty, `skills print` prints nothing and succeeds, because library code never panics (RUST-5) and a failure code for a tool without skills would serve no adopter. A tool whose skills print by name keeps its own print command, as ADR-0201 decided.

## Consequences

A tool mounts `skills` with one field, one match arm and three leaves. An adopter that keeps its own enum and match compiles unchanged, and one that switches regenerates its help goldens once, because the about lines and examples become generic. The failure codes of crumbkit-skills do not change.

Considered: a name lookup inside `print`, which adds a failure code to crumbkit-skills (ADR-0201); a constant for each leaf with fixed paths, which does not fit a tool that mounts `skills` under another group; one function that takes a sink, which every tool would then call with the standard streams.
