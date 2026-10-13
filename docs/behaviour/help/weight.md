# Help weight

This page covers `cargo crumbkit weigh`, which measures the bytes that each
path through a tool's help costs a reader.
[cargo-crumbkit](../../../crates/cargo-crumbkit/README.md) describes the
binary.

## cargo-crumbkit

The published crate cargo-crumbkit holds the binary `cargo-crumbkit`, so
`cargo crumbkit weigh --bin <NAME>` measures a tool. cargo-crumbkit is a
crumbkit tool itself: it parses with `crumbkit::parse`, prints the
`crumbkit/weigh/v1` document through `finish`, mounts `commands list`, has
its own failure codes, and runs crumbkit-test's checks on its own help.

Why: ADR-0033.

## Building and running the tool

`weigh` builds the named binary with
`cargo build --workspace --bin <NAME> --message-format json` and takes the
executable from cargo's messages. cargo's standard error goes to `weigh`'s
own, so a person sees cargo's progress lines and compiler errors as with any
cargo subcommand. `weigh` reads the command tree from
`<binary> commands list --all --json --full`, and a tool without
`commands list` fails with `crumbkit::tool::no_commands_list`, whose hint
says how to mount it ([commands list](commands-list.md)). The failures of
building and running the binary have `crumbkit::tool::*` codes, which
`cargo crumbkit docs` shares ([reference](reference.md)).

`weigh` runs `<binary> <path> -h` and `<binary> <path> --help` for each
visible command and counts the bytes on standard output. Each run has a
cleared environment and fresh XDG directories, as crumbkit-test's
`check_examples` gives each example, so `weigh` never reads a person's
aliases or writes to their usage log.

cargo-crumbkit's tests that run `weigh` give its cargo a target directory of
their own under `CARGO_TARGET_TMPDIR`. `weigh` builds the binary with that
package's features alone, while a test build unifies the features of every
package and its dev-dependencies. A build in the shared target directory
would therefore replace a binary that other tests run, and Windows cannot
replace a running executable. The example test builds shelf in its
directory before the examples run, so no example waits for a build.

Why: ADR-0033, ADR-0005.

## Output weight

A command's output weight is the largest size of one example's golden files,
which `weigh` reads from `--goldens <DIR>` or from `tests/golden/examples` in
the binary's package. crumbkit-test's `output_weight` reads it and names each
file with `example_golden_name`, the function that `check_examples` uses, so
the two cannot disagree. An example's size is its `.txt` golden file plus its
`.stderr` golden file, because an agent reads the notes on standard error as
well as the result. A command without a golden file has no output weight,
shown as `-` in text and `null` in JSON.

A command's path total is the sum of every `-h` page from the root down to
the command plus the command's output weight. crumbkit-test's `path_totals`
computes it, and its `check_path_weights` budgets the same sums
([help checks](checks.md#path-budgets)).

Why: ADR-0033, ADR-0025, ADR-0050.

## Usage figures

`--usage` adds the figures of the tool's [usage log](../usage-log/README.md)
to each row: `runs`, `help_before`, the help pages that sessions read before
the command, and `p95_output`, the 95th percentile of its real output.
`weigh` runs `<binary> self figures list --all --json --log <FILE>` with the
temporary XDG directories, so the tool reads the log with its own reader and
record checks, and its run never writes to the log it reads. A tool without
`self figures list` fails with `crumbkit::weight::no_usage_figures`.

The log is `--log <FILE>`, which implies `--usage`, or else `usage.jsonl` in
the tool's directory under the person's XDG state directory, which `weigh`
finds with the etcetera call that the usage log uses. `--since <DURATION>`
passes through to the tool and needs `--usage` or `--log`. A command that the
log does not name has 0 runs. Without `--usage`, the three fields are `null`.

Why: ADR-0033, ADR-0034.

## The weigh table

The compact columns are `path`, `short_help`, `output`, `path_total` and the
three usage figures. `long_help` is behind `--long`, because an agent reads
`--help` only when it needs the details.

Every weight is a byte count. The text table prints byte counts and each
command's full path, because crumbkit's table prints numbers as JSON does and
has no tree drawing. The JSON rows are in depth-first order from the root
with their fields in a fixed order, so CI or an agent can compare two runs by
diffing the output.

Why: ADR-0033.
