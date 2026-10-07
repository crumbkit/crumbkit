# 0009. Lints

- `[workspace.lints]` sets every lint to warn except `unsafe_code`, which is forbidden. CI fails on any warning through `CARGO_BUILD_WARNINGS=deny`.
- Code suppresses a lint with `#[expect]` and a reason, never `#[allow]`.
- `clippy.toml` confines environment, stdout, stderr, process and file-write calls to the module that owns each.

Considered: `#![deny(warnings)]`, `RUSTFLAGS="-D warnings"`, cargo-spellcheck.
