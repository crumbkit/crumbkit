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
- `Page` holds the flags `--limit <N>` and `--all`, which choose how many rows
  a listing prints. A command adds them to its own arguments with
  `#[command(flatten)]`, and its const parameter sets the default limit, which
  is 25 unless the command names another.
- `Listing` holds one page of rows, the total number of rows that matched, and
  an optional phrase that names them. `Listing::new` takes a page and a total
  that the tool counted itself, so a tool with a database never loads a row it
  will not print. `Listing::from_all` keeps the first page of rows that the
  tool already holds in memory.
- `finish` prints a result in the format that the flags choose and returns the
  exit code. `finish_to` does the same into a `Sink` over any two writers, so
  a test reads the output from memory.

A tool wires these parts into its own `main`:

```rust
use std::process::ExitCode;

use clap::Parser;
use crumbkit::{Document, Listing, Output, Page, Row};
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

const BOOKS_LIST: Document<Listing<Book>> = Document::new("shelf", "books-list", 1);

/// List the books in a small catalogue.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    output: Output,
    #[command(flatten)]
    page: Page<2>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let books = vec![
        Book { id: 1, title: "Pride and Prejudice", year: 1813 },
        Book { id: 2, title: "Frankenstein", year: 1818 },
        Book { id: 3, title: "Moby-Dick", year: 1851 },
    ];
    let listing = Listing::from_all(books, cli.page.limit()).described("books");
    crumbkit::finish(&cli.output, BOOKS_LIST, listing)
}
```

Without flags, this tool prints a table of the first two books with the
columns `ID` and `TITLE`, then the footer `Showing 2 of 3 books. --all shows
every one.` `--long` adds `YEAR`, and `--all` prints all three books without
the footer. With `--json`, it prints one line holding a JSON object whose
`schema` is `shelf/books-list/v1`, whose `data` is the array of books, and
whose `total` is 3.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
