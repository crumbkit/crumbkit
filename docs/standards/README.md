# Standards

Each rule states what crumbkit's own code does, why, and the gate that enforces
it: a lint, an xtask check, or review where no tool reaches. The decision
records in `docs/decisions/` explain the choices behind the rules.

## Rust

- **RUST-1.** Code builds without warnings, because a warning left in the build hides the next one. Gate: `CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets`, with the lints in the root `Cargo.toml`, which every package inherits through `[lints] workspace = true`.
- **RUST-2.** Code contains no `unsafe`, because crumbkit needs none and forbidding it rules out memory-safety bugs in crumbkit's own code. Gate: rustc's `unsafe_code` at `forbid`.
- **RUST-3.** Code suppresses a lint only with `#[expect(lint, reason = "...")]`, because the reason records why, and `#[expect]` warns when the lint stops firing. Gate: clippy's `allow_attributes` and `allow_attributes_without_reason`.
- **RUST-4.** Environment, process, standard stream and file-write calls happen only in the module that owns each effect, which suppresses the lint with `#[expect]`; other code takes a writer or a value, so its tests run in memory. Gate: clippy's `disallowed_methods` with the list in `clippy.toml`, and `print_stdout`, `print_stderr` and `dbg_macro`.
- **RUST-5.** Code outside tests returns errors and never panics, because a panic in a library ends the adopter's process. Gate: clippy's `unwrap_used`, `expect_used`, `panic`, `todo` and `unimplemented`, which `clippy.toml` allows in tests.
- **RUST-6.** A module with children is `foo.rs` beside a `foo/` directory, never `foo/mod.rs`, so each file name says which module it holds. Gate: clippy's `mod_module_files`.
- **RUST-7.** Unit tests live in `foo/tests.rs`, declared in `foo.rs` with `#[cfg(test)] mod tests;`, and each test name is a sentence, so code and tests read separately. Gate: review.
- **RUST-8.** A `.rs` file has at most 600 lines, so a person or an agent can read it whole. Gate: `cargo xtask check` counts each file's lines and fails, naming every file over 600; split such a file into modules.
- **RUST-9.** `rust-toolchain.toml` pins the stable toolchain and only a pull request of its own moves it, so a new clippy release never fails an unrelated change. The MSRV is the workspace `rust-version`, 1.89. Gate: review, and `cargo +1.89.0 check --workspace --all-targets` for the MSRV.

## Documentation

- **DOCS-1.** An `AGENTS.md` has at most 200 words, because an agent reads it at the start of every session, and it holds only what no tool or other document tells the agent. Gate: `cargo xtask check` counts whitespace-separated words and fails, naming every file over 200.
- **DOCS-2.** The Markdown files directly inside one directory of the `docs/` tree total at most 5,000 words, counting code blocks and tables, so each directory can be read in one sitting. Gate: `cargo xtask check docs` counts each directory on its own and fails, naming every directory over 5,000.
- **DOCS-3.** Every Markdown file under `docs/decisions/` is a decision record whose first line is `# ADR-NNNN. Title`, numbers are unique across the whole tree, and code and docs cite a record as `ADR-NNNN`, and each directory's `README.md` links to every record in it, so one search finds every reference to a decision and every record can be reached from the root. Gate: `cargo xtask check docs` fails on a missing title, a reused number, a citation of a number that no record uses, a record its node does not list, or a node link to a missing file.
