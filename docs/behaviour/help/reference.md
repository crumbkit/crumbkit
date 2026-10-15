# Command reference

This page covers the Markdown reference of every command of a tool, which
crumbkit-markdown renders and `cargo crumbkit docs` writes.
[crumbkit-markdown](../../../crates/crumbkit-markdown/README.md) describes
`render`.

## The reference

crumbkit-markdown's `render` reads the `crumbkit/commands-list/v1` document
that `<binary> commands list --all --json --full` prints and returns one
CommonMark file ([commands list](commands-list.md)). The reference therefore
needs no access to the tool's code and says what the tool's help says.
crumbkit-markdown depends on crumbkit only for `Effects::line`,
`Effects::writes_line`, `help::STREAM_LINE` and `help::HEADED_LINE`, so the
effects, writes, stream and headed lines use help's own wording.

The file opens with the title `Commands of <root>` and a line saying that
crumbkit-markdown writes it from the tool's help. Each command then has a
section in the document's depth-first order. The heading is the command line
in a code span. The section holds these parts:

- the long description and a `Commands` list that links to each subcommand;
- an `Arguments` list, whose items end with the facts that help shows:
  whether the argument is required or what can be given in its place,
  whether it takes several values, its possible values, its defaults, and
  whether it works after every subcommand;
- the examples as fenced `sh` blocks;
- the columns and the columns that `--long` adds, or for a command whose
  `form` is `headed` the JSON fields that `--fields` selects, under a
  heading that says so;
- the effects line, the writes line, and the stream line or the headed
  line;
- an `Exit codes` list.

The reference escapes the inline markup characters of help text and the
block markers at the start of a line, and keeps its code spans and line
breaks. Help text is plain text, so a character such as `*` or `<` prints as
written.

A link to a command uses the anchor that GitHub gives its heading: the text
lowercased, spaces turned into dashes, and other punctuation dropped.
`render` fails when two commands, or a command and the title, have one
anchor, because a link to the second would reach the first. It also fails on
a document that holds one page of a longer listing, so the reference never
leaves a command out.

Why: ADR-0044.

## Writing and checking the reference

`cargo crumbkit docs --bin <NAME>` builds the binary as `weigh` does, runs it
with a cleared environment and fresh XDG directories, and writes the
reference to `docs/commands.md` in the binary's package, or to
`--out <FILE>`. It prints the `crumbkit/docs/v1` document, which names the
file and the number of commands. Its failures share the `crumbkit::tool::*`
codes of building and running the binary with `weigh`
([help weight](weight.md)), and add `crumbkit::docs::unrenderable` and
`crumbkit::docs::write_failed`.

crumbkit-test's `check_command_reference` runs the binary's `commands list`
in the same way, renders the reference and compares it with the checked-in
file. `SNAPSHOTS=overwrite` writes the file, as it does for every golden file
([golden files](../checks/golden-files.md)). A tool that checks in its
reference adds this test, so a change to its help that leaves the reference
stale fails CI. The shelf example checks in `examples/shelf/docs/commands.md`.

Why: ADR-0044, ADR-0033, ADR-0004.
