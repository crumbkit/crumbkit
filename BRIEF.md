# Brief: a Leaf can carry notes printed after its examples in help

You are a Claude Code cloud session. This run is also a trial of whether a
cloud session can build and test this repository unaided.

## The change

- Add notes to a `Leaf`. Help for that command prints the notes after its
  examples.
- The purpose: a command with rules about its own output can state them in
  help without using clap's `after_help`.
- Follow the existing way a `Leaf` carries its examples, in naming, docs and
  tests.
- A `Leaf` with no notes prints the same help as today.

## How to work

- Read `AGENTS.md` first and follow it.
- Work on a new branch named `leaf-notes`, cut from `main`. Leave `BRIEF.md`
  out of it.
- Write the failing test first, then the code.
- Install the toolchain pinned in `rust-toolchain.toml`.
- Before you push, run these and make them pass:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets`
  - `cargo nextest run --workspace`
  - `cargo test --workspace --doc`
- Add a `CHANGELOG.md` entry if `AGENTS.md` asks for one.

## The pull request

- Open one pull request against `main`, marked ready, and wait for its CI to
  pass. Fix what CI finds.
- The commit message and the title describe the behaviour.
- Add no attribution lines and name no AI model anywhere.
- Name no company, product or repository outside the crumbkit organisation.
- Never merge the pull request.
- End the body with a section "Environment report" that lists each tool you
  had to install, each command that failed and why, and anything the
  environment blocked.

## When you are stuck

- Ask nobody. Stop, and write what blocked you in the pull request body, or
  in your final message when no pull request exists.
