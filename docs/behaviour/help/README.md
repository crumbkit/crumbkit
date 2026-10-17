# Help

This page covers what a tool's help pages show when the tool parses with
[`parse`](../../../crates/crumbkit/src/parse.rs): the lines crumbkit adds,
goal headings, runtime leaves, the word `help` and help topics. The other
pages in this directory cover [leaf examples and columns](examples.md),
[`commands list`](commands-list.md), the
[command reference](reference.md), [help weight](weight.md) and the
[help checks](checks.md).

## Lines crumbkit adds to a help page

`parse` keeps clap's help template for each command and adds crumbkit's
lines after the command's own `after_help` and `after_long_help`, so a page
keeps clap's layout and the tool's own text. crumbkit changes only the
commands that the typed words pass through, and it never clones the command
tree. The rustdoc of [`parse_to`](../../../crates/crumbkit/src/parse.rs)
gives an example of each line.

The lines print in this order, each only on the pages named:

1. `Examples:` and, in `--help`, `Columns:` on a leaf, which
   [Leaf examples and columns](examples.md) describes.
2. `Effects:` on a leaf that declares its effects, then `Writes:` on a leaf
   that declares write guards ([write guards](../effects/write-guards.md)),
   then `Output:` on a leaf that prints a stream, bytes, a headed batch or
   rows whose text is one line of JSON for each row
   ([streams](../output/streams.md),
   [headed bytes](../output/README.md#headed-bytes),
   [rows printed whole](../output/README.md#rows-printed-whole)), then `Problems:` on a leaf whose row
   type names a problem field.
3. In `-h`, one line that names the visible global flags of the root and of
   each command on the way. `--help` lists those flags in full.
4. On the root, a `Topics:` line that names the tool's topics and
   `exit-codes`.
5. On the clap root, when the guide opts in, a line that names a person's
   aliases ([aliases](../parsing/aliases.md)).
6. A `Next:` line. The help of a command with a visible subcommand ends with
   a `Next:` line that opens one of them. A leaf's `-h` ends with a `Next:`
   line that points to `--help` for its columns.

crumbkit's lines appear on guided and unguided pages alike. Help and the
version keep clap's colours only when standard output is a terminal and the
guide keeps [terminal detection](../output/terminal.md) on. Help, the
version, a topic and the topic list print to standard output with exit code 0.

Why: ADR-0021, ADR-0024, ADR-0039, ADR-0041, ADR-0047, ADR-0049, ADR-0065.

## The walk before parsing

crumbkit finds the deepest command the caller reached by walking the typed
words once before clap parses them. The walk reads each option's values from
clap's `Arg`, so an option's value that equals a subcommand name names no
command. The same walk finds `-h`, `--help`, `--json` and the word `help`
where clap would read them as flags or names.

[`command_path`](../../../crates/crumbkit/src/parse/walk.rs) makes the walk
public. It returns the path of the command that a tool's words name,
including on a line that clap refuses, without building or cloning the tree.
crumbkit-test's twin check reads each line's command with it, and
crumbkit-test keeps its own `split` for the words of a recovery.

How `parse` refuses a command line, including the one-line message and the
JSON refusal, is on the [parsing](../parsing/README.md) page.

Why: ADR-0021.

## Goal headings

A tool may list a group's subcommands under goal headings that it declares
in a `const` [`Guide`](../../../crates/crumbkit/src/help.rs), in place of
clap's single `Commands:` list. `parse` applies the headings to the groups
that the typed words pass through. [`help::extend`](../../../crates/crumbkit/src/help.rs)
applies them to the whole tree, for a tool that parses with plain clap and
for tests, through the same per-group function. The rustdoc of `extend`
states the layout of the list.

- A guided command gets clap's default template with the headed list in
  place of clap's argument sections, so a guided page has no `Arguments:`
  section, and the tool's own `before_help` and `before_long_help` still
  print at the top of the page.
- A list that holds `{` goes in `before_help` instead, because clap expands
  each `{tag}` in a template, and that command loses the tool's own
  `before_help`.
- A guided list ends a description with the command's visible aliases, as
  clap's `Commands:` list does, such as `[alias: ls]`.
- A command that the guide does not name keeps clap's `Commands:` list.
- `parse` and `extend` never fail on a guide.
  [`check_guide`](../../../crates/crumbkit-test/src/guide.rs) catches a guide
  that does not match the tree.

Why: ADR-0021, ADR-0027.

## Runtime leaves

A tool that adds commands to its tree at run time, such as one for each file
a person writes, gives them examples with
[`RuntimeLeaf`](../../../crates/crumbkit/src/help/runtime.rs)s.
`Guide::with_runtime_leaves` joins them after the `const` leaves into a
guide that every reader takes as it takes a `const` guide. The join leaks
the runtime leaves for the rest of the process. A runtime leaf has examples
and no document, and it takes effects and refusal alternatives. The joined
guide keeps the guide's renames and every other setting.

Why: ADR-0021, ADR-0024.

## The help command

`parse` answers the word `help` itself and turns off clap's generated `help`
subcommand, so no `Commands:` list shows a `help` line. It answers `help`
before clap parses, at every level below a command with subcommands, so the
tool's own type never sees the word. A tool's own command named `help`
still takes the word, and a command without subcommands reads `help` as an
ordinary word.

- `<tool> help` lists the tool's topics and then `exit-codes`.
- `<tool> help <TOPIC>` prints one topic.
- Every other `help` command line becomes the same command line with
  `--help`, so `shelf help books show` prints what `shelf books show --help`
  prints. An unknown word gets clap's usual refusal with its suggestions.

crumbkit-test's [`check_grammar`](../../../crates/crumbkit-test/src/grammar.rs)
reserves the word `help` for asking for help.

Why: ADR-0023.

## Topics

A tool declares its topics in its `Guide`, each a
[`Topic`](../../../crates/crumbkit/src/help/topic.rs) of a name, a one-line
summary and plain text. Topic text is a literal or comes from
`include_str!`. `parse` reads the topics from the guide, so the command tree
holds no topics. Topics belong to the root, which
[`Guide::root`](../../../crates/crumbkit/src/help.rs) can move for a tool
that another program runs with fixed words.

crumbkit adds its own topic, `exit-codes`, which lists crumbkit's codes 0, 1
and 2 and then the codes that the tool declares
([exit codes](../failures/exit-codes.md)). It comes last in the topic list
and on the root's `Topics:` line. A root has the topic, and answers `help`,
when it has subcommands or the tool declares topics, so a root without
either reads `help` as an ordinary word. A tool's topic named `exit-codes`
never prints, and `check_guide` rejects one.

The topic list and a topic's text print as plain text, also on a terminal.
The root's help names the topics on one line and gives no summaries.

Why: ADR-0023, ADR-0037.
