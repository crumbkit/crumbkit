# Working on crumbkit

Start at [docs/README.md](docs/README.md) when you are unsure where something
is or why it is that way.

A change is ready when these pass:

```sh
cargo test --workspace
cargo test -p crumbkit
cargo test -p crumbkit --features miette
CARGO_BUILD_WARNINGS=deny cargo clippy --workspace --all-targets
CARGO_BUILD_WARNINGS=deny cargo clippy -p crumbkit --all-targets
CARGO_BUILD_WARNINGS=deny cargo clippy -p crumbkit --all-targets --features miette
CARGO_BUILD_WARNINGS=deny cargo clippy -p crumbkit --all-targets --features tracing
cargo fmt --check
cargo xtask check
cargo +1.89.0 check --workspace --all-targets --all-features
cargo deny check bans licenses sources
cargo shear
zizmor .
```

- Follow `docs/standards/`. Before changing a choice that `docs/decisions/`
  records, edit that record in place; git keeps its history.
- Write the test first, and name it as a sentence.
- Give each fact one home (ADR-0013), and link to it elsewhere.
- Give every new dependency a reason in `Cargo.toml` (ADR-0003).

## Writing

Docs, comments and commit messages use plain declarative sentences with a
named subject. Avoid "not X but Y", fragments, metaphors and closing
flourishes. A stated limit says what it counts and what happens when it is
hit. Docs describe the project as it is now. A commit message describes the
behaviour that changed.
