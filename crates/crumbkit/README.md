# crumbkit

crumbkit provides conventions for Rust command-line tools built on clap, so
that people and agents can drill into a tool one level at a time. A tool keeps
its own clap types and its own `main`, and adopts only the parts it wants.
crumbkit is in design and is not published yet.

## Adding it

A tool adds crumbkit beside clap, serde and schemars, from this repository
until the first release:

```toml
[dependencies]
crumbkit = { git = "https://github.com/crumbkit/crumbkit" }
```

## Example

This tool lists books as a text table or, with `--json`, as a JSON
document, pages the list with `--limit` and `--all`, and prints a failure
with a stable code and a next step:

```rust
use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use crumbkit::help::Guide;
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
    let parsed = crumbkit::parse::<Cli>(Cli::command(), &Guide::new(), SPELLING);
    let (mut cli, command) = match parsed {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    cli.output.set_command(command);
    let result = list(&cli);
    crumbkit::finish(&cli.output, SPELLING, BOOKS_LIST, result)
}
```

The shelf example tool shows the exact output of a fuller tool in its
[golden files].

## Where to read more

- Output: [`Output`], [`finish`], [`Row`], [`Document`], [`Listing`],
  [`Page`] and [`Batch`], with [ADR-0016].
- Failures: [`Failure`], [`Recovery`], [`Code`], [`Spelling`] and
  [`Error`], with [ADR-0001].
- Parsing and help: [`parse`], [`help`] and [`Call`], with [ADR-0021],
  [ADR-0023] and [ADR-0024].
- The command listing for agents: [`commands`], with [ADR-0026].
- Per-id lookups: [`fetch_each`].
- The `miette` feature: the `diagnostic` method of [`Failure`] and the
  conversion of a `miette::Report` into [`Error`], with [ADR-0001].
- The `usage-log` and `tracing` features: [`usage`] and [`trace`], with
  [ADR-0029] and [ADR-0030].
- The JSON Schemas of crumbkit's own documents: [schemas], with [ADR-0007].
- The tests that check a tool against these conventions: [crumbkit-test].

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[golden files]: ../../examples/shelf/tests/golden
[`Output`]: src/output.rs
[`finish`]: src/output.rs
[`Row`]: src/row.rs
[`Document`]: src/document.rs
[`Listing`]: src/listing.rs
[`Page`]: src/page.rs
[`Batch`]: src/batch.rs
[`Failure`]: src/failure.rs
[`Recovery`]: src/failure.rs
[`Code`]: src/code.rs
[`Spelling`]: src/code.rs
[`Error`]: src/failure.rs
[`parse`]: src/parse.rs
[`help`]: src/help.rs
[`Call`]: src/call.rs
[`commands`]: src/commands.rs
[`fetch_each`]: src/fetch.rs
[`usage`]: src/usage.rs
[`trace`]: src/trace.rs
[schemas]: schemas
[crumbkit-test]: ../crumbkit-test/README.md
[ADR-0001]: ../../docs/decisions/api/0001-error-model.md
[ADR-0007]: ../../docs/decisions/api/0007-json-schemas.md
[ADR-0016]: ../../docs/decisions/api/output/0016-result-formats.md
[ADR-0021]: ../../docs/decisions/api/help/0021-help-pages.md
[ADR-0023]: ../../docs/decisions/api/help/0023-help-command-and-topics.md
[ADR-0024]: ../../docs/decisions/api/help/0024-leaf-examples-and-columns.md
[ADR-0026]: ../../docs/decisions/api/help/0026-branch-export.md
[ADR-0029]: ../../docs/decisions/api/usage/0029-usage-log.md
[ADR-0030]: ../../docs/decisions/api/usage/0030-trace-context.md
