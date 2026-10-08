# crumbkit

crumbkit provides conventions for Rust command-line tools built on clap, so
that people and agents can drill into a tool one level at a time. A tool keeps
its own clap types and its own `main`, and adopts only the parts it wants.

crumbkit is in design and is not published yet. This crate prints a command's
result in one of two formats: a compact text table for people and agents, and
a JSON document for programs. It prints a failure with a stable code and a
next step in the same two formats.

- `Row` marks a type that prints as one table row. It names the compact
  columns and the columns that `--long` adds.
- `Document` is the typed id of a JSON document, such as
  `shelf/books-list/v1`. The JSON format prints the id under `schema` and the
  result under `data`.
- `Output` holds the flags `--json`, `--long`, `--full` and `--fields`. A tool
  adds them to its own clap parser with `#[command(flatten)]`, and after
  parsing passes the command tree it parsed against to
  `Output::set_command`.
- `Page` holds the flags `--limit <N>` and `--all`, which choose how many rows
  a listing prints. A command adds them to its own arguments with
  `#[command(flatten)]`, and its const parameter sets the default limit, which
  is 25 unless the command names another.
- `Listing` holds one page of rows, the total number of rows that matched, and
  an optional phrase that names them. `Listing::new` takes a page and a total
  that the tool counted itself, so a tool with a database never loads a row it
  will not print. `Listing::from_all` keeps the first page of rows that the
  tool already holds in memory.
- `Call` names a command by its path from the root and the clap ids of the
  arguments to show. `Call::render` turns it into text, such as
  `shelf books show <ID>...`, against the `clap::Command` the tool built at
  runtime, and returns a `CallError` when a command or argument on it is
  missing or hidden.
- `Listing::next` gives a listing a next step: a `Call` and a phrase that
  the text output prints after the table and any footer, such as
  `Next: shelf books show <ID>... takes up to 100 ids in one call.` The line
  is left out when no command is set or the call does not resolve, and the
  JSON document never holds it.
- `Failure` extends `std::error::Error` with a stable `Code`, a `Recovery`
  and an optional url. A tool implements it for each of its error types.
- `Code` has three segments: the owning component, the area and the reason,
  such as `shelf`, `books` and `not_found`. `Code::new` is a `const fn`, so a
  malformed code fails to compile. `Code` has no `Display`.
- `Spelling` turns a `Code` into text. `Spelling::COLONS` prints
  `shelf::books::not_found`, `DOTS` prints `shelf.books.not_found`, `SLASHES`
  prints `shelf/books/not_found`, and `Spelling::custom` takes a tool's own
  function. A tool passes one spelling to `finish` and the same one to
  crumbkit-test's `check_codes`.
- `Recovery` is the caller's next step: `Run` a `Call`, `Retry` later, follow
  a `Hint`, or `None` with a reason.
- `Error` holds any `Failure`, and every `Failure` converts into it, so `?`
  works across a tool's error types.
- `finish` takes the handler's `Result` and returns the exit code. It prints
  an `Ok` result in the format that the flags choose and exits with code 0.
  It prints an `Err` as a failure on standard error, or with `--json` as one
  `crumbkit/failure/v1` document on standard output, and exits with code 1.
  `finish_to` does the same into a `Sink` over any two writers, so a test
  reads the output from memory.

A tool wires these parts into its own `main`:

```rust
use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use crumbkit::{Call, Code, Document, Failure, Listing, Output, Page, Recovery, Row, Spelling};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema)]
struct Book {
    id: u32,
    title: &'static str,
    author: &'static str,
    year: u16,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["id", "title"];
    const LONG_COLUMNS: &'static [&'static str] = &["year"];
}

const BOOKS_LIST: Document<Listing<Book>> = Document::new("shelf", "books-list", 1);
const SPELLING: Spelling = Spelling::COLONS;
const UNKNOWN_AUTHOR: Code = Code::new("shelf", "books", "unknown_author");

/// The catalogue has no book by the author the caller named.
#[derive(Debug)]
struct UnknownAuthor(String);

impl fmt::Display for UnknownAuthor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the catalogue has no books by {}", self.0)
    }
}

impl std::error::Error for UnknownAuthor {}

impl Failure for UnknownAuthor {
    fn code(&self) -> &'static Code {
        &UNKNOWN_AUTHOR
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&[]))
    }
}

/// List the books in a small catalogue.
#[derive(Parser)]
#[command(name = "books")]
struct Cli {
    /// List only the books by this author
    #[arg(long)]
    author: Option<String>,
    #[command(flatten)]
    output: Output,
    #[command(flatten)]
    page: Page<2>,
}

fn list(cli: &Cli) -> Result<Listing<Book>, UnknownAuthor> {
    let books: Vec<Book> = vec![
        Book { id: 1, title: "Pride and Prejudice", author: "Jane Austen", year: 1813 },
        Book { id: 2, title: "Frankenstein", author: "Mary Shelley", year: 1818 },
        Book { id: 3, title: "Moby-Dick", author: "Herman Melville", year: 1851 },
    ];
    let books: Vec<Book> = books
        .into_iter()
        .filter(|book| cli.author.as_deref().is_none_or(|author| book.author == author))
        .collect();
    if let (Some(author), true) = (&cli.author, books.is_empty()) {
        return Err(UnknownAuthor(author.clone()));
    }
    Ok(Listing::from_all(books, cli.page.limit())
        .described("books")
        .next(Call::new(&[]).arg("author"), "lists one author's books."))
}

fn main() -> ExitCode {
    let mut cli = Cli::parse();
    cli.output.set_command(Cli::command());
    let result = list(&cli);
    crumbkit::finish(&cli.output, SPELLING, BOOKS_LIST, result)
}
```

Without flags, this tool prints a table of the first two books with the
columns `ID` and `TITLE`, then the footer `Showing 2 of 3 books. --all shows
every one.`, then the line `Next: books --author <AUTHOR> lists one author's
books.` `--long` adds `YEAR`, and `--all` prints all three books and the
`Next:` line without the footer. With `--json`, it prints one line holding a
JSON object whose `schema` is `shelf/books-list/v1`, whose `data` is the
array of books, and whose `total` is 3.

With `--author 'Charles Dickens'`, it prints nothing on standard output and
exits with code 1. Standard error holds three lines: `error: the catalogue
has no books by Charles Dickens`, then `` help:  run `books` `` and
`code:  shelf::books::unknown_author`, each indented by two spaces. With
`--json` as well, standard output holds one line with a JSON object whose
`schema` is `crumbkit/failure/v1`, whose `code` is
`shelf::books::unknown_author`, and whose `recovery` is
`{"kind":"run","call":"books"}`.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
