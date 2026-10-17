# Commands list

This page covers `<tool> commands list [PATH]...`, which lists one branch of
a tool's command tree with each command's arguments, examples and columns,
so an agent loads a subtree in one call instead of reading its help page by
page. [`crumbkit::commands`](../../../crates/crumbkit/src/commands.rs)
describes `CommandsList` and how a tool mounts it.

## Mounting the command

The tool owns the wiring and adds the command to its own clap types. `parse`
adds no command and answers none, so a tool that does not want the export
leaves it out, and the command's help, goal heading, leaf and examples are
the tool's like any other command's. `commands` is a resource and `list` its
verb, and `CommandsList` flattens `Page`, so the command passes
crumbkit-test's tree checks with no opt-out. The command sits at the root,
because the not-found recovery runs `commands list` below the root.

Why: ADR-0026.

## What it lists

`list` reads the command tree that the tool passed to `Output::set_command`,
which is the tree that `parse` returns, so the listing shows the commands the
tool kept at run time. It builds that whole tree through `Output::command`
before it reads it, so it also sees each command that a `Command::defer`
closure adds. It fails with a fixed code when no command is set.

- A hidden command and everything below it are left out, as help leaves them
  out. A hidden command fails as an unknown one does, so the failure never
  reveals a command that the tool hid.
- The rows leave out the help and version flags, which are the same on every
  command and which `parse` changes, so the rows are the same whether or not
  the tree went through `parse`. A global argument is listed once.
- An argument's environment variable is left out, because crumbkit does not
  enable clap's `env` feature and so cannot read it.

The two failure codes of `commands list` are fixed by crumbkit, as
`crumbkit::cli::usage` is, and are not listed in a `CODES` list.

Why: ADR-0026, ADR-0001.

## Rows

Every row has every field, with `null` or an empty list for what a command
lacks, so a program reads every row the same way.

- `effects` holds the leaf's effects, or `null` when it declares none
  ([effects](../effects/README.md#declaring-effects)).
- `writes` holds the write guards, or `null` for a read-only command or one
  without guards ([write guards](../effects/write-guards.md)).
- `form` is `document`, `stream`, `bytes`, `headed` or `json-rows` for a
  leaf with a document and `null` for one without
  ([streams](../output/streams.md), [bytes](../output/shapes/README.md#bytes),
  [headed bytes](../output/shapes/README.md#headed-bytes),
  [rows printed whole](../output/shapes/README.md#rows-printed-whole)). A leaf has
  the form `json-rows` when its text prints each row as one line of JSON.
- `line` holds the form of each row's own text line, such as
  `<NAME>: <ABOUT>`, or `null` when the leaf's rows print a table or the
  leaf has no document
  ([rows with their own text line](../output/shapes/README.md#rows-with-their-own-text-line)).
- `columns` lists a headed leaf's JSON fields that `--fields` selects,
  ending with `text` and `base64`, because its text has no columns.
- `exits` lists the exit codes that the tool declares for the command, each
  with its code and meaning. crumbkit's own 0, 1 and 2 are the same for every
  command, so the rows leave them out ([exit codes](../failures/exit-codes.md)).

The text table shows only each command's path and about text, because the
nested lists would print as compact JSON in a table cell, and it ends with a
note that names every code and `<tool> help exit-codes`. `--json --full`
prints the arguments, examples, columns and exit codes. The schema of the
`crumbkit/commands-list/v1` document is checked in
([schemas](../schemas/README.md)). `commands list` prints only crumbkit's two
formats, and crumbkit-markdown writes a Markdown [reference](reference.md)
from its JSON document.

Why: ADR-0026, ADR-0016, ADR-0037, ADR-0039, ADR-0041, ADR-0047, ADR-0064,
ADR-0065.
