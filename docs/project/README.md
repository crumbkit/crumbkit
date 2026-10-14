# Project

These pages describe how the crumbkit repository is laid out, built,
checked, tested and released. [Behaviour](../behaviour/README.md) describes
what crumbkit does for a tool. Each section ends with the records that give
its reasons.

- [Dependencies and platforms](dependencies.md): what each dependency and
  feature adds, the MSRV, and the supported platforms.
- [Checks](checks.md): lints, the xtask checks and the supply-chain checks.
- [Testing](testing.md): golden files, end-to-end tests, property tests and
  performance budgets.
- [CI](ci.md): the pull-request lane, the scheduled lane and the ruleset.
- [Releases](releases.md): versions, the changelog, release pull requests
  and publishing.
- [Documentation](documentation.md): where each kind of fact lives.

## Repository layout

crumbkit is a virtual workspace.

| Path | Holds |
| --- | --- |
| `crates/crumbkit-core` | The published crate of the types that need no clap. crumbkit depends on it and re-exports its items. |
| `crates/crumbkit` | The published runtime crate. |
| `crates/crumbkit-test` | The published crate of convention checks. |
| `crates/cargo-crumbkit` | The published crate of the `cargo crumbkit` subcommand, which measures a tool's help and writes its command reference. |
| `crates/crumbkit-aliases` | The published crate that expands a person's aliases. |
| `crates/crumbkit-picker` | The published crate of the `Menu` picker. |
| `crates/crumbkit-markdown` | The published crate that writes a tool's command reference. |
| `examples/shelf` | An unpublished example package with a `shelf` binary. |
| `examples/reading` | An unpublished example package with a `reading` binary that adds a command for each reading list at run time. |
| `examples/tutorial` | An unpublished package with one `pantry` binary for each step of the [tutorial](../tutorial.md). |
| `benches` | The unpublished benchmark package. |
| `xtask` | The unpublished crate of repository checks. |

`Cargo.lock` is committed, and each published crate lists its files in
`include`. Modules and unit tests follow the file layout of RUST-6 and
RUST-7, so each file name says which module it holds, and code and tests
read separately.

Why: ADR-0014, ADR-0032.
