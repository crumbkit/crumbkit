# ADR-0010. Standards and xtask

- Each rule in `docs/standards/` has an ID, such as `RUST-4`, and names the gate that enforces it.
- An unpublished `xtask` crate runs the repository checks, and a `#[test]` calls each one. xtask parses its arguments with clap.
- `cargo xtask check` enforces the line limit of RUST-8, so a person or an agent can read each `.rs` file whole, and the word limit of DOCS-1, because an agent reads an `AGENTS.md` at the start of every session. It lists files with `git ls-files`, including untracked files that are not ignored.
- `cargo xtask check docs` enforces the directory word limit of DOCS-2, so each directory of the docs tree can be read in one sitting, and the record checks of DOCS-3. The tree includes `docs/` itself and every subdirectory. ADR-0017 says how the decision records grow within the word limit.
- `cargo xtask ci-time` measures the CI-3 limit from the run times that `gh api` returns for `ci.yml`.
- `cargo xtask run-time` builds shelf in release mode and times its whole runs against the PERF-2 limit, with an empty environment and XDG directories under `target/run-time/`.

Considered: shell scripts, just, cargo-make, an argument parser written by hand.
