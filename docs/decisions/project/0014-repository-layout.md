# ADR-0014. Repository layout

- crumbkit is a virtual workspace with published crates under `crates/`, an unpublished `examples/shelf/` package with a `shelf` binary, and `xtask/`.
- `Cargo.lock` is committed, and published crates list their files in `include`.
- Modules use `foo.rs` beside `foo/`, and unit tests live in a sibling `tests.rs`.

Considered: a root package, shelf as a crate example, inline test modules.
