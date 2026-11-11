# cargo-crumbkit

cargo-crumbkit adds the `cargo crumbkit` subcommand, which measures and
describes a tool built with crumbkit. `cargo crumbkit weigh` counts the
bytes that an agent reads on each path through the tool's help, from the
root down to each command. `cargo crumbkit docs` writes a Markdown
reference of every command. `cargo crumbkit tool-skill draft` and
`tool-skill check` draft and check the tool's agent skill.

## Adding it

A tool's author installs cargo-crumbkit from crates.io:

```sh
cargo install cargo-crumbkit
```

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
`--out` names. The reference for crumbkit's example tool is
[shelf's reference]. crumbkit-test's
`check_command_reference` fails a test when the checked-in file no longer
matches the tool's help.

## Checking the agent skill

`cargo crumbkit tool-skill check --bin shelf` builds `shelf` as `weigh` does and
reads every Markdown file under `skill` in shelf's package, or the file or
folder that `--skill` names. Each inline code span and fenced-block line
that starts with `shelf` must name real commands, their flags, a value for
each flag that takes one, and an accepted value where the argument lists
its values. Placeholders such as `ID` count as values. The text prints
`<FILE>:<LINE>: <SPAN>: <REASON>` for each failing line, and the run exits
with 1 when one fails, so CI can run it.

## Drafting the agent skill

`cargo crumbkit tool-skill draft --bin shelf` rewrites the lines between
`<!-- crumbkit:generated NAME -->` markers in shelf's `skill/SKILL.md`, or
in the file that `--skill` names, and leaves the author's text alone. It
writes the description, a table of the jobs that the help's goal headings
name, and, with `--usage <DIR>` of exported usage, the most-used commands.
`--check` writes nothing and exits with 1 when a draft would change the
file.

## Its own skill

`cargo crumbkit skills install` installs cargo crumbkit's agent skill, which
says what each command is for and how to audit a tool for places that
should use crumbkit. The audit writes a JSON report that
[the report schema] describes.

## Where to read more

- `cargo crumbkit weigh --help` describes each column.
- The [weigh docs], the [command reference docs], the [skill check docs]
  and the [skill draft docs] describe `weigh`, the command reference,
  `tool-skill check` and `tool-skill draft`, and [ADR-0033], [ADR-0044],
  [ADR-0193], [ADR-0194] and [ADR-0199] record their reasons.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[shelf's weights]: tests/golden/examples/cargo-crumbkit-weigh-bin-shelf.txt
[schema]: schemas/weigh-v1.json
[ADR-0033]: ../../docs/decisions/api/help/0033-weigh.md
[shelf's reference]: ../../examples/shelf/docs/commands.md
[ADR-0044]: ../../docs/decisions/api/help/0044-command-reference.md
[weigh docs]: ../../docs/behaviour/help/weight.md
[command reference docs]: ../../docs/behaviour/help/reference.md
[skill check docs]: ../../docs/behaviour/help/skill-check/README.md
[ADR-0193]: ../../docs/decisions/api/help/skill/0193-skill-check.md
[skill draft docs]: ../../docs/behaviour/help/skill-draft/README.md
[ADR-0194]: ../../docs/decisions/api/help/skill/README.md
[the report schema]: skill/audit-report-v1.json
[ADR-0199]: ../../docs/decisions/api/help/skill/0199-skill-files.md
