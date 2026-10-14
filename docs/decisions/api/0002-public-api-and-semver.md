# ADR-0002. Public API and semver

- Date: 2026-10-07
- Status: accepted

## Context

Adopters build their tools on crumbkit's types, and an agent reads a
tool's exit codes and failure codes as a contract. crumbkit needed rules
that let its API grow without breaking adopters, and fixed meanings for
the exit codes and codes that callers depend on.

## Decision

- Each release pull request ran cargo-semver-checks against the last
  published version and went through the Rust API Guidelines checklist,
  so a breaking change is caught before it is published.
- crumbkit re-exported crumbkit-core's items at their old paths, so
  crumbkit-core's API is part of crumbkit's.
- Configuration types got private fields and `const fn` builders, so crumbkit can add settings without a breaking change. Types that hold data read at run time got
  ordinary builders, because a `const fn` cannot grow a `Vec`.
  `RuntimePlace` became a separate type from `RuntimeFailing`, so a place
  passed where a command line belongs is a compile error. Enums that may
  grow are `#[non_exhaustive]`, and traits that adopters implement gain
  only methods with defaults.
- The keys of `#[failure(...)]` and the code the derive writes for them
  became public API, because adopters' code depends on what each key
  returns.
- Handlers took an arguments struct that derives `clap::Args`,
  `Serialize` and `JsonSchema` and never see `ArgMatches`.
- Failure codes are never reused. A success exits with 0, a failure with 1 and a refusal
  with 2 by default, and a tool may map any failure to its own code from 3
  to 125. Codes from 126 up stay with the shell.
- `parse` returned a `Stop` in place of a bare `ExitCode`, so a tool can
  still ask whether the caller wanted JSON. Its `Output` is boxed, because
  a large error type is flagged by clippy and the box costs one allocation
  only on a run that `parse` ends.
- The `complete` feature followed clap_complete's unstable dynamic
  completion, so what it offers may change in a minor release and the
  changelog names each such change.

## Consequences

The semver checks start at 0.1.1, after the first published version.
`docs/behaviour/adoption/README.md` lists the current API promises.

Considered: exit codes 1 and 2 fixed for crumbkit's failures, a place that is a `RuntimeFailing` without a code and panics when it is run, a `RuntimeFailing` generic over a marker type for a place and a line, constructors generic over borrowed and owned codes, `parse` returning an `ExitCode` and a separate function that finds `--json`, a `Stop` with its own `is_json` and no `Output`, public-field structs, a feature-gated schema bound, `Call` methods hidden from the docs, holding the `complete` feature back until clap_complete's dynamic completion is stable.
