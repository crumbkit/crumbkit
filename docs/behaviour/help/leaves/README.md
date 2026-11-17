# Leaf notes and the quoting of command lines

This page covers the notes that a leaf's help prints after its examples, and
how help quotes the command lines it prints. The rustdoc of
[`Leaf`](../../../../crates/crumbkit/src/help/leaf.rs) states the contract of
`Leaf::notes`. [Leaf examples and columns](../examples.md) covers the rest of
a leaf's help.

## Leaf notes

A command whose output follows rules of its own, such as which rows it leaves
out, declares them with `Leaf::notes`. Each note is one line of text.

- `-h` and `--help` print a `Notes:` section right after `Examples:`, before
  the `Columns:` sections of `--help` and before crumbkit's lines.
- Each note prints on its own line, indented two spaces as an example line
  is. A note does not wrap at the terminal width.
- A leaf with notes and no examples prints `Notes:` alone.
- `help::extend` and `parse` build the examples and notes block with one
  function, so a tool that calls `extend` before `parse` prints each note
  once. A tool's own `after_help` prints before the block.
- `commands list`, the command reference and skill drafts read only
  examples and columns, so they show no notes. A `RuntimeLeaf` has no notes.
- crumbkit-test's `check_help` fails a note that is empty or holds a line
  break, and names the leaf and the note.

Why: ADR-0250.

## Quoting of command lines

crumbkit prints each command line with
[`help::quote`](../../../../crates/crumbkit/src/help/words.rs), which quotes a
word for a POSIX shell. On Windows, help prints the same POSIX quoting, so a
line pasted into PowerShell or `cmd` may need other quoting.

Why: ADR-0024, ADR-0003.
