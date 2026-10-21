# ADR-0137. A row type describes its child lines in help

- Date: 2026-10-21
- Status: accepted
- Supersedes: ADR-0120 and ADR-0135, which it amends
- Missed: ADR-0120 fixed the help sentence of child lines and ADR-0135 kept it, so help called the lines indented at an indent of 0 and described one line per item for a row whose child lines fold several items into one.

## Context

ADR-0120 prints a fixed help line for a row type that sets `CHILDREN`:
`Children: one indented line under each row for each of its <FIELD>.`.
ADR-0135 added `CHILD_INDENT`, and with an indent of 0 the line still says
"indented". An adopting tool has a workspace row that prints one child
line for each application. When every application is stopped, the row
prints one folded line, such as `api, worker · all stopped`, in place of
those lines. The fixed sentence therefore misdescribes what the row prints,
and the tool asked for a way to supply the sentence.

## Decision

- `Row` gained the const `CHILDREN_HELP: Option<&'static str>`, which
  defaults to `None`. It holds the text that help prints after
  `Children: `, ending with a full stop. Help keeps the `Children:` label,
  so every help line still starts with crumbkit's own label.
- Without the text, help prints crumbkit's sentence. The sentence is
  `one indented line under each row for each of its <FIELD>.` when
  `CHILD_INDENT` is above 0, and `one line under each row for each of its
  <FIELD>.` when it is 0. Every existing row type prints the same bytes.
- `commands list` gained the nullable field `children_help`, which holds
  the text that help prints after `Children: `, the row type's own text or
  crumbkit's sentence, and is null when `children` is null. An agent reads
  the description there without rebuilding it from the indent.
- `help::children_line` takes that text and returns the whole line.
  crumbkit-markdown prints the line from `children_help`, escaped as other
  help text is, so the field name in crumbkit's sentence no longer prints
  in a code span. A document without `children_help` prints no
  `Children:` line.
- `Leaf::children_help` returns the row type's own text, and
  `Leaf::children_text` returns the text that help prints. crumbkit-test's
  `check_opt_outs` lists a `children` customisation with crumbkit's
  sentence, or with the row type's text after the field it describes.
- crumbkit-test's `check_guide` fails a text that is empty, holds a control
  character or does not end with a full stop, because help prints it as one
  sentence on one line, and a text on a row type without `CHILDREN`,
  because help then never prints it.
- The API-1 ground is one adopter's request for an ordinary need, because
  any row whose child lines are not one line per item needs help to say
  what they are (ADR-0122).

## Consequences

A row type whose child lines fold, group or summarise items describes them
in help, `commands list` and the Markdown reference with one text. A row
type at column 1 no longer has help that calls its lines indented. crumbkit
cannot check that the text is true of `child_lines`, so the tool keeps the
two in step. `docs/behaviour/output/shapes/README.md` and
`docs/behaviour/help/README.md` describe the current behaviour.

Considered: a text that replaces the whole line, which would let a tool drop
or misspell the `Children:` label; a format string with a `{field}`
placeholder, which crumbkit would have to parse; `commands list` fields for
the indent and the row type's text, so the reference could rebuild the
sentence with the field in a code span, which makes every reader of the
listing rebuild it too; a check that `CHILD_INDENT` 0 comes with a text,
which would fail every such row type for a sentence crumbkit now words
correctly; and the names `CHILDREN_LINE`, which reads as one of the
`child_lines`, `CHILDREN_ABOUT` and `CHILD_LINES_HELP`. `CHILDREN_HELP`
sits beside `CHILDREN` and says that help prints it.
