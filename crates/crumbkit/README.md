# crumbkit

crumbkit provides conventions for Rust command-line tools built on clap, so
that people and agents can drill into a tool one level at a time. A tool keeps
its own clap types and its own `main`, and adopts only the parts it wants.

## Adding it

A tool adds crumbkit beside clap, serde and schemars:

```toml
[dependencies]
crumbkit = "0.1"
```

Before 1.0, a minor release, such as 0.2, may change the API. The
[changelog] names each change.

## Example

This tool lists books as a table or as JSON, pages them, and prints a
failure with a stable code and a next step:

```rust
use std::fmt;
use std::process::ExitCode;

use clap::{CommandFactory, Parser};
use crumbkit::help::Guide;
use crumbkit::{Call, Code, Document, Failure, Listing, Page, Recovery, Row, ViewFlags};
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

const BOOKS_LIST: Document<Listing<Book>> =
    Document::new("shelf", "books-list", 1, "One page of the books.");
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
    view: ViewFlags,
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
    let parsed = crumbkit::parse::<Cli>(Cli::command(), &Guide::new());
    let (cli, output) = match parsed {
        Ok(parsed) => parsed,
        Err(stop) => return stop.exit_code(),
    };
    let result = list(&cli);
    crumbkit::finish(output, BOOKS_LIST, result)
}
```

The [tutorial] builds a tool step by step, and shelf's [golden files] show
a fuller one.

## Where to read more

The [behaviour docs] describe each part in full.

- Output: [`Output`], [`finish`], [`Row`], [`Document`], [`Listing`],
  [`Page`], [`Batch`] and [`Prose`].
- Value types of your own: [`Shown`] and [`Duration`].
- Failures: [`Failure`], [`Recovery`], [`Code`], [`Spelling`] and
  [`Error`].
- Parsing and help: [`parse`], [`help`] and [`Call`].
- The listings for agents: [`commands`] and [`schemas`].
- Per-id lookups: [`fetch_each`].
- The `miette` feature: the `diagnostic` method of [`Failure`] and the
  conversion of a `miette::Report` into [`Error`].
- Run reports, usage log and tracing: [`events`], [crumbkit-usage], [`trace`].
- Handing the terminal to another program: [crumbkit-child].
- The `color` feature: help with clap's styles in a [`Sink`] that asks for
  them.
- The `complete` feature: shell completion through `crumbkit::complete`,
  which a minor release may change.
- The JSON Schemas of crumbkit's own documents: [schemas].
- Tests that check a tool against these conventions: [crumbkit-test].
- The `derive` feature: `#[derive(Failure)]`, which writes a [`Failure`]
  impl from attributes ([crumbkit-derive]).

Licensed under Apache-2.0 or MIT, at your option.

[tutorial]: ../../docs/tutorial.md
[golden files]: ../../examples/shelf/tests/golden
[`Output`]: src/output.rs
[`finish`]: src/output.rs
[`Row`]: src/row.rs
[`Document`]: src/document.rs
[`Listing`]: src/listing.rs
[`Page`]: src/page.rs
[`Batch`]: src/batch.rs
[`Prose`]: src/prose.rs
[`Shown`]: src/shown.rs
[`Duration`]: src/duration.rs
[`Failure`]: src/failure.rs
[`Recovery`]: src/failure.rs
[`Code`]: src/code.rs
[`Spelling`]: src/code.rs
[`Error`]: src/failure.rs
[`parse`]: src/parse.rs
[`Sink`]: src/sink.rs
[`help`]: src/help.rs
[`Call`]: src/call.rs
[`commands`]: src/commands.rs
[`schemas`]: src/schemas.rs
[`fetch_each`]: src/fetch.rs
[`events`]: src/events.rs
[crumbkit-usage]: ../crumbkit-usage/README.md
[crumbkit-child]: ../crumbkit-child/README.md
[`trace`]: src/trace.rs
[schemas]: schemas
[crumbkit-test]: ../crumbkit-test/README.md
[crumbkit-derive]: ../crumbkit-derive/README.md
[behaviour docs]: ../../docs/behaviour/README.md
[changelog]: ../../CHANGELOG.md
