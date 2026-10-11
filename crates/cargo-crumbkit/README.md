# cargo-crumbkit

cargo-crumbkit adds the `cargo crumbkit` subcommand, which measures a tool
built with crumbkit. `cargo crumbkit weigh` counts the bytes that an agent
reads on each path through the tool's help: each command's `-h` and `--help`
pages, the output of its examples, and the total from the root down to the
command. crumbkit is in design and is not published yet.

## Adding it

A tool's author installs cargo-crumbkit from this repository until the
first release:

```sh
cargo install --git https://github.com/crumbkit/crumbkit cargo-crumbkit
```

The `main` branch can change without notice before the first release.

## Example

In a workspace whose binary `shelf` mounts crumbkit's `commands list`, this
command builds `shelf` and prints one row for each visible command:

```sh
cargo crumbkit weigh --bin shelf
```

The output for crumbkit's example tool is pinned in [shelf's weights].
`--json` prints the same rows as the `crumbkit/weigh/v1` document, which the
[schema] describes, so CI or an agent can compare two runs.

`weigh` runs the tool's binary with a cleared environment and fresh XDG
directories, so it never reads the person's aliases or writes to their usage
log. It reads each example's output weight from the golden files that
crumbkit-test's `check_examples` writes, in the directory that `--goldens`
names, or in `tests/golden/examples` in the binary's package.

`--usage` adds each command's runs, the help pages that callers read before
it and the 95th percentile of its real output, from the tool's own usage
log. `--log <FILE>` reads another copy of a log, and `--since <DURATION>`
sets how far back to read. The tool needs crumbkit's `usage-log` feature
and a `self figures list` command, as shelf has. The log stays on the
machine, and `weigh` only reads it.

## Where to read more

- `cargo crumbkit weigh --help` describes each column.
- [ADR-0033] records the design and the alternatives considered.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[shelf's weights]: tests/golden/examples/cargo-crumbkit-weigh-bin-shelf.txt
[schema]: schemas/weigh-v1.json
[ADR-0033]: ../../docs/decisions/api/help/0033-weigh.md
