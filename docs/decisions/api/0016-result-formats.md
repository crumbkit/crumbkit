# ADR-0016. Result formats

- Results come in two formats: compact text for people and agents, and JSON for programs. The text table already has a header row, one record per line and no repeated keys.
- Markdown is used only to export help, through `schema cli --markdown`.
- A type that prints as a table row implements `Row`. Its `COLUMNS` are the compact columns in display order, its `LONG_COLUMNS` are the columns that `--long` adds after them, and both hold serialized field names. A field in neither list appears only with `--json --full`.
- `finish` takes one `Row` or a `Vec` of rows through the sealed `Rows` trait. A single row's JSON `data` is an object, and a `Vec`'s is an array.
- The text table has a header row with each column name in upper case, then one line per row, and an empty result prints the header only. Each column is as wide as its widest cell in characters, two spaces separate the columns, and the last column is not padded, so no line ends in spaces.
- A cell shows a string as it is, a number or boolean in its JSON form, `null` or a missing field as `-`, and an array or object as compact JSON. A control character in a string shows in its `char::escape_default` form, such as `\n` or `\u{1b}`, so each row stays on one line.
- A tool adds the `Output` flags with `#[command(flatten)]`, and they are global. `--json` prints the document on one line, `--long` adds the long columns in text and JSON, and `--full` requires `--json` and prints every field in declaration order. `--fields a,b` prints exactly the named fields in that order, conflicts with `--long` and `--full`, and exits with code 2 on a name that is not a property of the row type's JSON Schema.
- `finish` writes through a `Sink`, which owns standard output and standard error, and `finish_to` writes into a given sink, so tests run in memory. A pipe that the reader closed early exits with code 0, and any other write error exits with code 1.

Considered: TOON, a Markdown result format, unicode-width for cell widths, serde_json's `preserve_order` feature for column order.
