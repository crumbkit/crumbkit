# ADR-0010. Standards and xtask

- Each rule in `docs/standards/` has an ID, such as `RUST-4`, and names the gate that enforces it.
- An unpublished `xtask` crate runs the repository checks, and a `#[test]` calls each one.
- `cargo xtask check` fails on a `.rs` file over 600 lines or an `AGENTS.md` over 200 words. It lists files with `git ls-files`, including untracked files that are not ignored.
- `cargo xtask check docs` fails when the Markdown files directly inside any directory of the docs tree total more than 5,000 words. The tree includes `docs/` itself and every subdirectory, and each directory is counted on its own. Words are whitespace-separated and include code blocks and tables. ADR-0017 says how the decision records grow within this limit.
- `cargo xtask check docs` also fails when a Markdown file under `docs/decisions/` does not start with an `# ADR-NNNN.` title, when two records share a number, or when any Rust or Markdown file cites an `ADR-NNNN` that no record defines.

Considered: shell scripts, just, cargo-make.
