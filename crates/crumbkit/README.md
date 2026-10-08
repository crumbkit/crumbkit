# crumbkit

crumbkit provides conventions for Rust command-line tools built on clap, so
that people and agents can drill into a tool one level at a time. A tool keeps
its own clap types and its own `main`, and adopts only the parts it wants.

crumbkit is in design and is not published yet. This crate prints a command's
result in one of two formats: a compact text table for people and agents, and
a JSON document for programs.

- `Row` marks a type that prints as one table row. It names the compact
  columns and the columns that `--long` adds.
- `Document` is the typed id of a JSON document, such as
  `shelf/books-list/v1`. The JSON format prints the id under `schema` and the
  result under `data`.
- `Output` holds the flags `--json`, `--long`, `--full` and `--fields`. A tool
  adds them to its own clap parser with `#[command(flatten)]`.
- `finish` prints a result in the format that the flags choose and returns the
  exit code. `finish_to` does the same into a `Sink` over any two writers, so
  a test reads the output from memory.

A tool wires these parts into its own `main`:

```rust
use std::process::ExitCode;

use clap::Parser;
use crumbkit::{Document, Output, Row};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
struct Book {
    id: u32,
    title: &'static str,
    year: u16,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
    const LONG_COLUMNS: &'static [&'static str] = &["year"];
}

const BOOKS_LIST: Document<Vec<Book>> = Document::new("shelf", "books-list", 1);

/// List the books in a small catalogue.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    output: Output,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let books = vec![
        Book { id: 1, title: "Pride and Prejudice", year: 1813 },
        Book { id: 2, title: "Frankenstein", year: 1818 },
    ];
    crumbkit::finish(&cli.output, BOOKS_LIST, books)
}
```

Without flags, this tool prints a table with the columns `ID` and `TITLE`.
`--long` adds `YEAR`. With `--json`, it prints one line holding a JSON object
whose `schema` is `shelf/books-list/v1` and whose `data` is the array of
books.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
