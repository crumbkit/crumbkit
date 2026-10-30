# Goal headings

This page covers how help lists a group's subcommands under headings that
the tool declares, and how root help lists the tool's root verbs.

## Goal headings

A tool may list a group's subcommands under goal headings that it declares
in a `const` [`Guide`](../../../../crates/crumbkit/src/help.rs), in place of
clap's single `Commands:` list. `parse` applies the headings to the groups
that the typed words pass through. [`help::extend`](../../../../crates/crumbkit/src/help.rs)
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
- A group built with `Group::compact` lists only names in `-h`: one line for
  each goal, with the heading and a colon, then the subcommand names
  separated by commas, such as `Read:    list, show`. The names start in one
  column, two spaces after the longest heading. Commands that no goal names
  follow on a final `Commands:` line. Its `--help` keeps the full list. The
  two lists go in `before_help` and `before_long_help`, which are the only
  parts of clap's template that differ between `-h` and `--help`, so a
  compact group's command loses the tool's own `before_help` and
  `before_long_help`. Its `-h` ends its help line with
  `(see more with '--help')`.
- A guided list ends a description with the command's visible aliases, as
  clap's `Commands:` list does, such as `[alias: ls]`.
- A command that the guide does not name keeps clap's `Commands:` list,
  unless it is a root with [root verbs](#root-verbs).
- `parse` and `extend` never fail on a guide.
  [`check_guide`](../../../../crates/crumbkit-test/src/guide.rs) catches a guide
  that does not match the tree.

Why: ADR-0021, ADR-0027, ADR-0102.

## Root verbs

A tool whose commands sit directly under the root on purpose, such as
`start` and `stop`, declares them with
[`Guide::root_verbs`](../../../../crates/crumbkit/src/help/root_verbs.rs)
and one reason for the whole list. The root's `-h` and `--help` then list
each declared verb that no goal of a root group names under the heading
`Verbs`, in the order of the declaration, after the root group's goals and
before the `Commands` heading of the other subcommands. A root without a
group of its own gets the two headings in place of clap's `Commands:` list,
as the reading example's root help shows:

```text
Usage: reading [OPTIONS] <COMMAND>

Verbs
  where     Print the directory that holds reading's lists

Commands
  entries   Work with the entries of any reading list
  lists     Print the files of reading's lists
```

A compact root group's `-h` lists them on a `Verbs:` line after its goals.
Only the clap root lists root verbs, and a guide that declares none keeps
the root's help as it was. The declaration's checks are on the
[declarations](../../adoption/declarations.md#root-verbs) page.

Why: ADR-0165.
