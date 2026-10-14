# Working on crumbkit

Start at [docs/README.md](docs/README.md) to find anything and why.

A change is ready when these pass:

```sh
cargo test --workspace
cargo test -p crumbkit
cargo test -p crumbkit --features miette
cargo test -p crumbkit-child
CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets
CARGO_BUILD_WARNINGS=deny cargo clippy -p crumbkit --all-targets
CARGO_BUILD_WARNINGS=deny cargo clippy -p crumbkit --all-targets --features miette
CARGO_BUILD_WARNINGS=deny cargo clippy -p crumbkit --all-targets --features tracing
CARGO_BUILD_WARNINGS=deny cargo clippy -p crumbkit-child --all-targets
cargo fmt --check
cargo xtask check
cargo +1.89.0 check --workspace --all-targets --all-features
cargo deny check bans licenses sources
cargo shear
zizmor .
```

- Follow `docs/standards/`. `docs/` and rustdoc describe crumbkit now.
- New Unreleased changelog entries need a trailer: `Docs-Swept: <id>`
  or `Docs-Owed: <id>; from: <old>; to: <new>; docs: <paths>` (DOCS-7).
- Dated decision records are frozen except Status and Children (ADR-0017).
- Write the test first, named as a sentence.
- Give each new dependency a reason in `Cargo.toml` (ADR-0003).

## Writing

Docs, comments and commit messages use plain declarative sentences with a
named subject. Avoid "not X but Y", fragments, metaphors and closing
flourishes. A stated limit says what it counts and what happens when it is
hit. Commit messages describe changed behaviour.
