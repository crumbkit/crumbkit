# ADR-0014. Repository layout

- crumbkit is a virtual workspace with published crates under `crates/`, an unpublished `examples/shelf/` package with a `shelf` binary, and `xtask/`. crumbkit depends on crumbkit-core, the published crate that holds the types that need no clap (ADR-0032).
- `Cargo.lock` is committed, and published crates list their files in `include`.
- Modules and unit tests follow the file layout of RUST-6 and RUST-7, so each file name says which module it holds, and code and tests read separately.

Considered: a root package, shelf as a crate example, inline test modules.
