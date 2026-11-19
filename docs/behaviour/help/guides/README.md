# Guide topics

A tool declares a topic as a task guide with
[`Topic::guide`](../../../../crates/crumbkit/src/help/topic.rs), a list of
`Step`s. Each step holds a one-line explanation and between 1 and 3 commands, each the
words after the tool's name. A guide has at most 12 steps.

`<tool> help <name>` prints a numbered line for each step and then each command
on an indented line that starts with `$`, with each word quoted as example
lines quote it:

```text
1. Sign in.
   $ records login --user you@example.com
2. Create a record.
   $ records create --title 'Quarterly report'
```

The root's `Topics:` line names a guide with the other topics. `Topic::text`
returns an empty string for a guide, because the printed text holds the tool's
name, and `Topic::render` returns what help prints for either kind.

## The check

[`check_help`](../../../../crates/crumbkit-test/src/help/guide.rs) parses every
command of every step with the tool's name in front. It fails naming the topic,
the step number, clap's error kind and the command line as help prints it. A
`--help` or `--version` command counts as refused, as for a leaf's examples. It
also fails a guide with no steps or with more than 12, a step with no commands
or with more than 3, and a step whose explanation is empty or holds a line
break. The check does not run the commands.

Why: ADR-0257.
