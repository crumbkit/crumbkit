# 0009. Lints

- `[workspace.lints]` sets every lint to warn except `unsafe_code`, which is forbidden. CI fails on any warning through `CARGO_BUILD_WARNINGS=deny`.
- Code suppresses a lint with `#[expect]` and a reason, never `#[allow]`. Clippy's `allow_attributes` and `allow_attributes_without_reason` lints enforce this.
- `rust-toolchain.toml` pins the stable toolchain, and a pull request of its own moves it to a newer release, so a new clippy release never fails an unrelated change.
- `clippy.toml` confines environment, stdout, stderr, process and file-write calls to the module that owns each.

Considered: `#![deny(warnings)]`, `RUSTFLAGS="-D warnings"`, cargo-spellcheck, an unpinned stable toolchain, an xtask suppression check.
