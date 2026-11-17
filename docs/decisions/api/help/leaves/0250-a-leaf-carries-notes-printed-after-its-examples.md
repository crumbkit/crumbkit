# ADR-0250. A leaf carries notes that help prints after its examples

- Date: 2026-11-17
- Status: accepted

## Context

A command whose output follows rules of its own, such as which rows it
leaves out or what a field holds, had no place for those rules on its own
help page. An adopting tool wrote a `Notes:` section with clap's
`after_help`. That text sits outside the tool record, so crumbkit-test could
not check it, and `help::extend` and `parse` treated it as the tool's own
text. A `Topic` would have moved the notes off the command's page.

## Decision

- `Leaf::notes` takes a slice of one-line notes, and `Leaf::get_notes`
  returns them.
- `-h` and `--help` print a `Notes:` section right after `Examples:`, before
  the `Columns:` sections and before crumbkit's lines. Each note prints on one
  line indented two spaces, as an example line is. A leaf with notes and no
  examples prints `Notes:` alone.
- One function, `leaf::leaf_text`, builds the examples and notes block for
  both `parse` and `help::extend`. `parse` compares the command's own
  `after_help` with that whole block, so a tool that called `extend` before
  `parse` prints its notes once.
- crumbkit-test's `check_help` fails a note that is empty or holds a line
  break, and names the leaf.
- The API-1 ground for the new `Leaf` setting is the adopting tool that wrote
  its notes in `after_help`, and shelf's `books remove`, which notes what
  removal changes.
- `commands list`, the command reference and skill drafts read examples and
  columns only, so they show no notes.

## Consequences

A leaf's notes are part of the tool record, so crumbkit-test checks them
with the leaf. `commands list` does not show them. A long note does not wrap at the terminal width, as an
example does not. A `RuntimeLeaf` has no notes.
`docs/behaviour/help/leaves/README.md` describes the current behaviour.

Considered: a `Topic`, which moves the notes off the command's page; a
`Leaf` setting that replaces `after_help` with free text, which crumbkit-test
could not check line by line; wrapping each note, which examples do not do.
