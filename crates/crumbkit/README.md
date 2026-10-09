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
- `parse` parses the command line in place of clap's `Cli::parse`, and
  returns the tool's parsed type and the command tree it parsed against, or
  the exit code for `main` to return. It takes the tool's `help::Guide`,
  and a tool without one passes `&Guide::new()`. It adds lines to clap's
  help pages:
  `-h` names the global flags on one line, which `--help` lists in full, and
  the help of a command with a visible subcommand ends with a `Next:` line
  such as `Next: shelf books <COMMAND> -h`. It answers `help` in place of
  clap's `help` subcommand: `shelf help books show` and
  `shelf books help show` print what `shelf books show --help` prints. It
  prints a refused command line as a
  failure with the code `crumbkit::cli::usage`, a recovery that runs the
  `-h` of the command the caller reached, and exit code 2. `parse_from`
  takes the arguments, and `parse_to` also takes a `Sink`, so a test reads
  the output from memory.
- A `help::Guide` gives a group's help goal headings that the tool
  declares, such as `Read` and `Change`, in place of clap's single
  `Commands:` list. A tool declares the guide once as a `const` of `Group`s,
  each a command path from the root with its `Goal`s, and each goal is a
  heading and the real names of its subcommands. A visible subcommand that
  no goal names is listed under a final `Commands` heading, and a command
  that the guide does not name keeps clap's list. `parse` applies the goal
  headings to the commands that the typed words pass through, and
  `help::extend` applies them to the whole tree for a tool that parses with
  plain clap. A guide also holds the tool's help topics, each a
  `help::Topic` with a name, a one-line summary and plain text, which
  `parse` reads: `shelf help` lists them, `shelf help catalogue` prints one,
  and the root's help names them. crumbkit enables clap's `help` feature,
  which help templates need.
- `Page` holds the flags `--limit <N>` and `--all`, which choose how many rows
  a listing prints. A command adds them to its own arguments with
  `#[command(flatten)]`, and its const parameter sets the default limit, which
  is 25 unless the command names another.
- `Listing` holds one page of rows, the total number of rows that matched, and
  an optional phrase that names them. `Listing::new` takes a page and a total
  that the tool counted itself, so a tool with a database never loads a row it
  will not print. `Listing::from_all` keeps the first page of rows that the
  tool already holds in memory.
- `Batch` holds the result of a command that takes many ids: the rows for
  the ids that worked, in the order given, and the failures for the ids
  that did not. A tool whose backend has a batch lookup, such as an SQL `IN`
  query, builds it with `Batch::new` from that one call. A handler returns
  `Result<Batch<R, E>, E2>`, where the outer `Err` is a failure of the whole
  command.
- `fetch_each` calls a per-id lookup for each id on at most a given number
  of threads, and returns one result per id in the order of the ids. A
  synchronous tool whose backend looks up one id per call uses it. An async
  tool uses its own bounded stream, and a tool that needs rate limits or
  retries writes its own loop. With one worker, one id or none, it runs
  every lookup in the calling thread and starts no thread.
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
  works across a tool's error types. `Error` implements `std::error::Error`,
  so a tool's `main` can pass it on to `Box<dyn std::error::Error>`.
- `finish` takes the handler's `Result` and returns the exit code. It prints
  an `Ok` result in the format that the flags choose and exits with code 0.
  It prints an `Err` as a failure on standard error, or with `--json` as one
  `crumbkit/failure/v1` document on standard output, and exits with code 1.
  It prints a `Batch`'s rows as a table on standard output and then each
  failure on standard error, or with `--json` one document with the rows
  under `data` and the failures under `failures`, and exits with code 1
  when the batch holds a failure. `finish_to` does the same into a `Sink`
  over any two writers, so a test reads the output from memory.

A tool wires these parts into its own `main`:

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

Without flags, this tool prints a table of the first two books with the
columns `ID` and `TITLE`, then the footer `Showing 2 of 3 books. --all shows
every one.`, then the line `Next: books --author <AUTHOR> lists one author's
books.` `--long` adds `YEAR`, and `--all` prints all three books and the
`Next:` line without the footer. With `--json`, it prints one line holding a
JSON object whose `schema` is `shelf/books-list/v1`, whose `data` is the
array of books, and whose `total` is 3.

With `--limt 1`, it prints nothing on standard output and exits with code
2. Standard error holds `error: unexpected argument '--limt' found; a
similar argument exists: '--limit'`, then `` help:  run `books -h` `` and
`code:  crumbkit::cli::usage`, when the tool enables clap's default
features.

With `--author 'Charles Dickens'`, it prints nothing on standard output and
exits with code 1. Standard error holds three lines: `error: the catalogue
has no books by Charles Dickens`, then `` help:  run `books` `` and
`code:  shelf::books::unknown_author`, each indented by two spaces. With
`--json` as well, standard output holds one line with a JSON object whose
`schema` is `crumbkit/failure/v1`, whose `code` is
`shelf::books::unknown_author`, and whose `recovery` is
`{"kind":"run","call":"books"}`.

## The miette feature

The `miette` feature, off by default, connects crumbkit to the miette error
library. A tool turns it on with `features = ["miette"]` on its crumbkit
dependency. The feature enables no miette features, so it adds no renderer
and no derive macro.

With the feature on, `Failure` gains the method `diagnostic`, which returns
`None` by default. A failure type that also implements `miette::Diagnostic`
returns `Some(self)`, and `finish` then prints each label of the diagnostic
as an `at:` line after the causes, with the source line that holds the
label's start and a caret underline:

```text
error: unknown key `colour`
  at:    shelf.toml:3:1: this key is not known
     3 | colour = "red"
       | ^^^^^^
  help:  use `color`
  code:  shelf::config::unknown_key
```

The line and column are 1-based, and the column counts characters. A label
whose span the source cannot read prints its byte offset, such as
`at:    byte 400`, and no source line. With `--json`, the failure document
and each batch failure hold the labels under `labels`, with `offset` and
`length` in bytes, and `name`, `line`, `column` and `text` when they are
known. crumbkit reads only the diagnostic's labels and source code, so the
code, the recovery and the url still come from `Failure`.

The feature also converts a `miette::Report` into `crumbkit::Error`, so `?`
works on a `miette::Result` in a handler that returns `crumbkit::Error`. The
converted failure's code is `crumbkit::report::unchecked` for every report,
the report's own code follows its message in parentheses, and the report's
help becomes a `Recovery::Hint`. A tool whose own failure type travels inside
a report calls `downcast` on the report before converting it, so the caller
sees the tool's own code.

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
