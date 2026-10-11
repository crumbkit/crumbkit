# ADR-0016. Result formats

- Results come in two formats: compact text for people and agents, and JSON for programs ([`finish`](../../../../crates/crumbkit/src/output.rs)). The text table already has a header row, one record per line and no repeated keys.
- No command prints Markdown. A Markdown export of help is decided with the documentation (ADR-0013), and `commands list` exports a branch of the command tree in these two formats (ADR-0026).
- A type that prints as a table row implements [`Row`](../../../../crates/crumbkit/src/row.rs), which names its compact columns and the columns that `--long` adds, and a field in neither list appears only with `--json --full`.
- The text table pads columns by characters and escapes control characters, so each row stays on one line and no line ends in padding. Cell widths count characters, so a wide character misaligns a column.
- The [`Output`](../../../../crates/crumbkit/src/output.rs) flags are global. An unknown `--fields` name is refused before anything prints (ADR-0001).
- A command that may show fewer rows than matched flattens [`Page`](../../../../crates/crumbkit/src/page.rs), so the paging flags belong to that command and its `--help` shows its own default limit. crumbkit implements `Serialize` and `JsonSchema` for `Page` by hand, so a build without the `usage-log` feature needs no serde or schemars derive macros. The `usage-log` feature enables both derives for the usage record (ADR-0029).
- Such a command returns a [`Listing`](../../../../crates/crumbkit/src/listing.rs). `Listing::new` takes a total that the tool counted itself, so a handler never loads a row it will not print.
- When a listing shows fewer rows than its total, a footer on standard output says so and names `--all`. crumbkit owns the wording, and the tool supplies only the description.
- A listing may carry one next step and a note, which print in text only. A next step whose `Call` does not resolve is left out, so crumbkit never prints a command the tool hid.
- A failure prints as text on standard error, with nothing on standard output, or with `--json` as one failure document on standard output.
- With the `miette` feature, each label of a failure's diagnostic prints with its source line and a caret underline, drawn by crumbkit.
- A command that takes many ids returns a [`Batch`](../../../../crates/crumbkit/src/batch.rs). The rows print on standard output and the failures on standard error, and with `--json` both go into one document (ADR-0007). A batch with at least one failure exits with code 1.
- `finish` writes through a [`Sink`](../../../../crates/crumbkit/src/sink.rs), and `finish_to` writes into a given sink, so tests run in memory. A pipe that the reader closed early keeps the exit code of the result.

Considered: a failure on standard output without `--json`, TOON, a Markdown result format, unicode-width for cell widths, serde_json's `preserve_order` feature for column order, global paging flags, the footer on standard error, the description in the JSON document, the note in the JSON document, deriving `Serialize` and `JsonSchema` for `Page`, failing a whole many-id command when one id fails, a summary line after a batch's failures, a batch's failures on standard output in text, a separate exit code for a batch in which some ids worked, carets measured with unicode-width.
