# ADR-0033. Help weight

- Date: 2026-10-11
- Status: accepted

## Context

A tool's builder needs to see what each path through the help costs an
agent, from the root page down to a command's output, and to compare the
estimate with what callers actually did. The measure has to come from the
built binary, because a tool may change its tree at run time.

## Decision

- The published cargo-crumbkit crate held `cargo crumbkit weigh`, itself
  a crumbkit tool that parses with `parse`, prints through `finish` and
  checks its own help.
- `weigh` built the named binary through cargo and read its tree from
  `commands list --all --json --full`, so it measures what the binary
  serves. Each run has a cleared environment and fresh XDG directories, so
  `weigh` never reads a person's aliases or writes to their usage log.
- A command's output weight was the largest example's golden files,
  standard output and standard error together, because an agent reads
  both. `weigh` and crumbkit-test share the functions that name and sum
  them, so the two cannot disagree.
- `--usage` added the figures from `self figures list`, run by the tool
  itself, so the log is read with the tool's own reader and record checks.
- Every weight was a byte count, and the JSON rows come in a fixed
  depth-first order, so CI or an agent can diff two runs.
- `weigh`'s tests used a target directory of their own, because a build
  with other features in the shared directory would replace a binary that
  other tests run, and Windows cannot replace a running executable.

## Consequences

`docs/behaviour/help/weight.md` describes the current columns and
options.

Considered: an HTML report, token counts, a `--compare` flag, an in-process library form that reads the clap tree without building the binary, sizes in KB, tree prefixes such as `├─` in the text table, a public usage-log reader in crumbkit that `weigh` calls in-process, passing the person's state directory to the tool's run, an optional value on `--usage`, `long_help` as a compact column, a nextest test group for the tests that run `weigh`, a longer time limit for the examples of `weigh`, a `weigh` option that takes a built binary, an output weight of standard output alone.
