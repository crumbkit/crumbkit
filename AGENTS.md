# Working on crumbkit

crumbkit is a Rust workspace of conventions for command-line tools built on
clap. These instructions apply to anyone changing it, person or agent.

## Build, test and check

`rust-toolchain.toml` pins the toolchain, and rustup installs it on first use.
A change is ready when all four commands pass:

```sh
cargo test --workspace
CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets
cargo fmt --check
cargo xtask check
```

`cargo xtask check docs` runs only the docs word check. The MSRV is 1.89, and
`cargo +1.89.0 check --workspace --all-targets` confirms it still builds.

## Where things are

- `crates/` holds the published crates, `examples/shelf/` the example tool,
  and `xtask/` the repository checks.
- `docs/decisions/` holds the decision records. Each record describes the
  current decision, and a change edits the record in place.
- `docs/standards/` holds the rules for code and docs. Each rule has an ID and
  names its gate.

## How to change code

- Write the test first and watch it fail. Name each test as a sentence, such
  as `a_file_over_600_lines_is_reported`.
- Put unit tests in a sibling `tests.rs`, as `foo.rs` and `foo/tests.rs`.
- Suppress a lint only with `#[expect(lint, reason = "...")]`.
- Add a dependency only with a reason in `Cargo.toml`, as decision 0003
  requires.

## Writing style

This applies to docs, comments and commit messages.

- Write plain declarative sentences with a named subject and the claim in the
  main clause.
- Do not use "not X but Y" constructions, sentence fragments, metaphors or
  closing flourishes.
- When you state a limit, say what it counts and what happens when it is hit.
- Describe the project as it is now. Git history keeps the past.
- A commit message describes the behaviour that changed, such as "cargo xtask
  check reports oversized files and docs directories".
