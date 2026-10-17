# ADR-0064. Rows with their own text line

- Date: 2026-10-17
- Status: accepted

## Context

crumbkit printed rows in text as a table with a header. Some tools print
each row in a form their users already read and parse, such as
`<POINTER> -> <TARGET>` for a link or `<NAME>: <ABOUT>` for a list. A
table of the same fields repeats a header that those users do not need and
changes the form that their scripts and habits rely on. A tool that wanted
its own form had to print bytes and lose `--fields`, `--long`, the budget
and the listing footer.

## Decision

- `Row` gained the const `LINE: Option<&'static str>`, which defaults to
  `None`, and the provided method `text_line(&self) -> Option<String>`,
  which defaults to `None`. `LINE` names the form of each row's line for
  help. `text_line` builds one row's line.
- When `LINE` is `Some` and the run passes none of `--json`, `--long` and
  `--fields`, the text prints each row's line with control characters
  escaped, followed by a line feed, and prints no header. A row whose
  `text_line` returns `None` prints as one line of compact JSON with every
  field.
- `--long` and `--fields` print crumbkit's table. JSON is unchanged.
- A stream of such rows prints no header and prints each row's line as it
  arrives.
- The output budget measures the lines, and a listing keeps its footer on
  standard error.
- Help prints the line `Text: one line per row, <LINE>; --fields and
  --long print a table.` after the output line. It names only `--fields`
  when the command does not offer `--long`. A headed leaf prints no such
  line.
- `commands list` gained the nullable field `line`, and crumbkit-markdown
  prints the text line in each command's section.
- crumbkit-test's `check_guide` fails a `LINE` that is not one non-empty
  line, and a headed leaf whose row type declares `LINE`. `check_opt_outs`
  lists a `text-line` customisation for each leaf whose row type declares
  `LINE`.

## Consequences

A tool keeps its own row form in text and still gets crumbkit's JSON,
`--fields`, `--long`, budget, footer and help. Two forms of text exist for
one row type, and a script that reads the default text reads the tool's
form. `docs/behaviour/output/README.md` states the rule.

Considered: a header above the lines, a `Display` bound on the row type,
printing a row without a line as a table row, and a separate result type
for lined rows.
