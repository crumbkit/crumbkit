# cargo-crumbkit

cargo-crumbkit adds the `cargo crumbkit` subcommand, which measures and
describes a tool built with crumbkit. `cargo crumbkit weigh` counts the
bytes that an agent reads on each path through the tool's help: each
command's `-h` and `--help` pages, the output of its examples, and the total
from the root down to the command. `cargo crumbkit docs` writes a Markdown
reference of every command. crumbkit is in design and is not published yet.

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
sets how far back to read. The tool needs to keep the log with crumbkit-usage
and to have a `self figures list` command, as shelf has. The log stays on the
machine, and `weigh` only reads it.

## Writing the command reference

`cargo crumbkit docs --bin shelf` builds `shelf` as `weigh` does and writes
the reference to `docs/commands.md` in shelf's package, or to the file that
`--out` names. crumbkit-markdown writes the reference from the tool's
`commands list --all --json --full` document, and the reference for
crumbkit's example tool is [shelf's reference]. crumbkit-test's
`check_command_reference` fails a test when the checked-in file no longer
matches the tool's help.

## Where to read more

- `cargo crumbkit weigh --help` describes each column.
- The [weigh docs] and the [command reference docs] describe `weigh` and
  the command reference, and [ADR-0033] and [ADR-0044] record their
  reasons.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[shelf's weights]: tests/golden/examples/cargo-crumbkit-weigh-bin-shelf.txt
[schema]: schemas/weigh-v1.json
[ADR-0033]: ../../docs/decisions/api/help/0033-weigh.md
[shelf's reference]: ../../examples/shelf/docs/commands.md
[ADR-0044]: ../../docs/decisions/api/help/0044-command-reference.md
[weigh docs]: ../../docs/behaviour/help/weight.md
[command reference docs]: ../../docs/behaviour/help/reference.md
