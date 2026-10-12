# ADR-0014. Repository layout

- crumbkit is a virtual workspace with published crates under `crates/`, unpublished example packages, `examples/shelf/` with a `shelf` binary and `examples/reading/` with a `reading` binary that adds a command for each reading list at run time, the unpublished `benches/` package of ADR-0008, and `xtask/`. crumbkit depends on crumbkit-core, the published crate that holds the types that need no clap (ADR-0032). cargo-crumbkit is the published crate of the `cargo crumbkit` subcommand, which measures a tool's help (ADR-0033).
- `Cargo.lock` is committed, and published crates list their files in `include`.
- Modules and unit tests follow the file layout of RUST-6 and RUST-7, so each file name says which module it holds, and code and tests read separately.

Considered: a root package, shelf as a crate example, inline test modules.
