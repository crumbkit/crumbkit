# 0010. Standards and xtask

- Each rule in `docs/standards/` has an ID, such as `RUST-4`, and names the gate that enforces it.
- An unpublished `xtask` crate runs the repository checks, and a `#[test]` calls each one.
- `cargo xtask check` fails on a `.rs` file over 600 lines or an `AGENTS.md` over 200 words. It lists files with `git ls-files`, including untracked files that are not ignored.
- `cargo xtask check docs` fails when the Markdown files directly inside any directory of the docs tree total more than 5,000 words. The tree includes `docs/` itself and every subdirectory, and each directory is counted on its own. Words are whitespace-separated and include code blocks and tables.

Considered: shell scripts, just, cargo-make.
