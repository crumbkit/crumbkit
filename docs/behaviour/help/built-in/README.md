# Built-in commands

Every row of [`commands list`](../commands-list.md) has `built_in`, so a
program that reads a tool's command tree, such as a page over the tool's
commands, can tell the commands that crumbkit and its add-on crates provide
from the tool's own.

## Which rows are built in

`built_in` is `true` on the row of a command whose leaf declares a document
or stream that crumbkit marks as its own: `commands list`, `schemas list`,
`schemas show`, `self usage`, `self figures list`, `self runs list`,
`self runs export`, `skills install`, `skills check`, `skills print`,
`completion install`, `completion check`, `completion print` and
`aliases list`. A group is built in when it has at least one visible command
below it and every one of them is built in, so a `self` group that also
holds the tool's own `self doctor` is not. Every other row has `false`.

A crumbkit command whose leaf the tool does not declare reads `false`, and
crumbkit-test's leaf checks require the leaf. The text table keeps its
columns, so the field appears with `--json`, `--full` or `--fields`.

## Where the mark comes from

The tool mounts the `commands`, `schemas`, `self`, `skills`, `completion`
and `aliases` groups in its own clap types, so the command tree cannot say who provides a
command. The document that a leaf declares is the one place in the tool
record that crumbkit's code chooses. crumbkit's crates mark their documents
with the hidden `Document::built_in` where they define them, and a tool
never calls it. `commands list` works out a group's mark on the same walk
that collects the branch, and never compares a group's name.

Why: ADR-0198, ADR-0026.

## A group builds when its word is typed

A tool writes `crumbkit::Lazy<S>` for the field of a built-in group, such as
`commands: crumbkit::Lazy<CommandsCommand>`, and calls `into_inner()` in its
match. `Cli::command()` then holds the group with no subcommands, and a
run that types another command never builds the group's arguments.

`parse` expands a group when the typed words name it, and again when the
words after `help` name it, before it reads the tree. `crumbkit::reach`
does the same for a tree that a tool reads before it parses. `Run::command`,
`help::extend`, completion and crumbkit-test's checks build or expand every
group. Help pages, refusals, `commands list` rows and completion scripts are
the same as with an eager group.

A tool's own code that reads `Cli::command()` without building it sees a
lazy group with no subcommands until it calls `reach` or `Command::build`.

Why: ADR-0246.
