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
break. It applies the view-flag rule of leaf examples to a command whose path
names a leaf of the tool record, so an unoffered `--long`, `--fields` or
`--full` fails.

## A step that points to another tool

`Step::see(about, tool, topic)` declares a step that another tool carries out,
such as signing in with a separate login tool. `<tool> help <guide>` prints the
explanation and the line `$ <tool> help <topic>` under it, so a reader or an
agent opens the other tool's guide from its installed binary:

```text
1. Sign in with the login tool.
   $ logintool help sign-in
```

The step counts toward the limit of 12 steps. It has no commands, so the limit
of 3 commands does not apply to it. `check_help` fails a pointer whose `tool`
is not lowercase letters and digits in words joined by single dashes, starting
with a letter, and one whose `topic` breaks the topic-name rule. It also fails
a `tool` that is the tool's own name, because a step in the same tool needs a
step with commands.

`check_help` does not run or parse the other tool's commands, because it holds
no other tool's command tree, and it does not know whether the other tool is
installed or has the topic. When the other tool removes the topic, no test in
this tool fails, and a reader gets that tool's unknown-topic refusal. The other
tool's own `check_help` keeps the topic's commands true.

The check skips these:

- A command whose path names no leaf, such as a bare group word. clap accepts
  it when the group does not require a subcommand, and at run time it prints
  the group's help.
- The command's output, and whether it succeeds when run. The check does not
  run the commands.
- Preconditions, which `Example::needs` holds for leaf examples.

Why: ADR-0257, ADR-0258.
