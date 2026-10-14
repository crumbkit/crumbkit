# crumbkit-core

crumbkit-core holds the types of [crumbkit] that need no clap: the failure
model, the typed ids of JSON documents, the calls that name a next step, and
the row and result types. A crate that is not a command-line tool, such as a
web server, depends on crumbkit-core alone and builds no clap. crumbkit
re-exports every item at the same path, so a tool built on crumbkit does not
add this crate. crumbkit is in design and is not published yet.

## Adding it

A crate adds crumbkit-core from this repository until the first release:

```toml
[dependencies]
crumbkit-core = { git = "https://github.com/crumbkit/crumbkit" }
```

The `main` branch can change without notice before the first release. The
`miette` feature adds the `diagnostic` method of [`Failure`] and converts a
`miette::Report` into [`Error`]. The `derive` feature adds
`#[derive(Failure)]`, which writes a [`Failure`] impl from attributes.

## Example

This failure has a stable code and a next step, and converts into
[`Error`] with `?`:

```rust
use std::fmt;

use crumbkit_core::{Call, Code, Error, Failure, Recovery, Spelling};

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");

/// The catalogue has no book with the id the caller gave.
#[derive(Debug)]
struct NoSuchBook(u32);

impl fmt::Display for NoSuchBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the catalogue has no book {}", self.0)
    }
}

impl std::error::Error for NoSuchBook {}

impl Failure for NoSuchBook {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Run(Call::new(&["books", "list"]))
    }
}

fn find(id: u32) -> Result<String, NoSuchBook> {
    Err(NoSuchBook(id))
}

fn show(id: u32) -> Result<String, Error> {
    let title = find(id)?;
    Ok(title)
}

let error = show(7).unwrap_err();
assert_eq!(error.to_string(), "the catalogue has no book 7");
assert_eq!(Spelling::DOTS.spell(error.failure().code()), "shelf.books.not_found");
```

## Where to read more

- Failures: [`Failure`], [`Recovery`], [`Code`], [`Spelling`] and
  [`Error`], with [failure docs].
- Results: [`Document`], [`Row`], [`Listing`], [`Batch`] and [`Prose`].
- Next steps: [`Call`], which crumbkit renders against a tool's commands.
- Why this crate exists and what it holds: [ADR-0032].

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.

[crumbkit]: ../crumbkit/README.md
[`Failure`]: src/failure.rs
[`Recovery`]: src/failure.rs
[`Code`]: src/code.rs
[`Spelling`]: src/code.rs
[`Error`]: src/failure.rs
[`Document`]: src/document.rs
[`Call`]: src/call.rs
[`Row`]: src/row.rs
[`Listing`]: src/listing.rs
[`Batch`]: src/batch.rs
[`Prose`]: src/prose.rs
[ADR-0032]: ../../docs/decisions/api/0032-core-crate.md
[failure docs]: ../../docs/behaviour/failures/README.md
