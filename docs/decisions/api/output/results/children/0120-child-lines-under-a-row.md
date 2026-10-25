# ADR-0120. Child lines under a row

- Date: 2026-10-20
- Status: amended by ADR-0135, ADR-0137, ADR-0138 and ADR-0151

## Context

An adopting tool lists services and wants each service's components, each
with a mark for its state, shown under the service in text. In JSON the
components must stay a typed array. A table cell that holds an array
prints compact JSON.

## Decision

- `Row` gained the const `CHILDREN: Option<&'static str>`, which defaults
  to `None`, and the provided method `child_lines(&self) -> Vec<String>`,
  which returns no lines by default. `CHILDREN` names the field that holds
  the children, and `child_lines` builds their lines.
- The text table and a row's own text line (ADR-0064) print each child
  line under the row, indented by two spaces, with control and format
  characters escaped. Child lines do not change the column widths.
  `--long` keeps them, a stream prints them after each item's line, and
  the output budget counts them.
- Compact JSON holds the field after the columns, and the compact schema
  requires it. `--fields` holds the field only when it is named, and its
  text then prints it as a cell and prints no child lines.
- Help prints `Children: one indented line under each row for each of its
  <FIELD>.` after the `Output:` and `Text:` lines. `commands list` gained
  the nullable field `children`, and crumbkit-markdown prints the line.
- crumbkit-test's `check_guide` fails a field that is not a documented
  field of the row type, a field that is also a column, and a headed leaf
  that sets `CHILDREN`. `check_opt_outs` lists a `children` customisation.

## Consequences

A tool shows nested items in text and keeps them typed in JSON. A script
reading the default text sees indented lines between table rows.

Considered: a nested table, and a cell that joins the children with
commas.
