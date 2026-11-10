# The program that started a run

A program that starts a crumbkit tool can mark the run with its own name, so
the tool's [usage log](../README.md) records which runs the program started
and the [usage report](../report.md) can leave them out. A page that shows a
tool's figures, and runs `self figures list` itself each time it loads the
tool, uses this to keep its own runs out of the figures it shows.

## Marking a run

The program sets the tool's variable `<TOOL>_PROGRAM` to its own name, such
as `SHELF_PROGRAM=flightdeck`. The prefix is spelled as the prefix of
`<TOOL>_OUTPUT_BUDGET` is, and `ToolName::variable("PROGRAM")` returns the
whole name. The log reads the variable when it writes the run's line, and
records the name in the line's optional `program` field, which the
checked-in [`usage-v1.json`](../../../../crates/crumbkit-usage/schemas/usage-v1.json)
describes. The record keeps its `v1` id.

A program name is 1 to 32 bytes of lowercase ASCII letters, digits, `-` and
`_`, starting with a letter. The program chooses the name for itself, so the
field never holds a value that a caller typed. The field adds at most 44
bytes to a line, which keeps within the line's limit of 1 KiB as the other
fields do.

The program sets the variable only on the runs that it starts for its own
purposes. A run that a person starts through the program carries no
variable, so every report counts it as the person's.

Why: ADR-0195, ADR-0029, ADR-0079.

## A value that is not a program name

When the variable is unset or empty, the line has no `program` field and the
run prints nothing about it. When the variable holds any other value that is
not a program name, or a value that is not UTF-8, the run prints one line on
standard error when it writes its log line, such as:

```text
warning: SHELF_PROGRAM was ignored, because a program name is 1 to 32 lowercase letters, digits, `-` and `_`, starting with a letter, such as flightdeck; the run is recorded without a program
```

The warning names the variable and the rule, and never the value, which may
hold anything. It goes to standard error, so `--json` output on standard
output stays one document. The run's line is still written, without a
`program` field, and every report counts the run. This is the one case in
which the usage log prints anything, because a program that misspells its
name would otherwise see its runs counted with no sign of why.

Why: ADR-0195.

## Leaving a program's runs out of the reports

`self usage`, `self figures list` and `self runs list` take
`--exclude-program <NAME>`, repeated or with names separated by commas. They
leave out every run whose `program` equals one of the names before they
count anything, so those runs add nothing to the runs, sessions, callers,
findings or figures, and the summary does not count them as skipped. A run
without a `program` field, and a run that another program marked, stays in.

A name that is not a program name is refused with `<tool>::cli::usage` and
exit 2, and the message states the rule, before the report reads the log.
The report checks each record's `program` against the same rule, as it
checks callers and codes, and skips a record that fails it and counts it in
the summary's `skipped`. `self runs export` prints every record with its
`program` field as stored, and leaves no run out.

shelf's end-to-end test in
[`tests/usage.rs`](../../../../examples/shelf/tests/usage.rs) plants a value
in `SHELF_PROGRAM` and in an argument of a run that a program marked, and
checks that no line holds either and that the marked run's line holds only
the program's name.

Why: ADR-0195, ADR-0034.
