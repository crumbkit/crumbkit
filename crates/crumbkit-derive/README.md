# crumbkit-derive

crumbkit-derive holds `#[derive(Failure)]`, which writes the `Failure` impl
of [crumbkit] for an enum or a struct from `#[failure(...)]` attributes. It
also writes a `CODES` constant that lists the type's codes for the twin and
recovery checks. It also holds `#[derive(Row)]`, which writes the `Row` impl
of a struct from `#[row(...)]` attributes on its fields. A tool does not add
this crate itself. It turns on the `derive` feature of crumbkit, which
re-exports each derive beside the trait of the same name.

## Adding it

```toml
[dependencies]
crumbkit = { version = "0.4", features = ["derive"] }
```

The feature is off by default, so a tool that writes its impls by hand does
not build crumbkit-derive. Such a tool builds syn either way, because
`clap_derive` needs it.

## Example

Each variant names its code and its next step:

```rust
use std::fmt;

use crumbkit::{Call, Code, Failure, Recovery};

const NOT_FOUND: Code<'static> = Code::new("shelf", "books", "not_found");
const REMOVED_FILE: Code<'static> = Code::new("shelf", "books", "removed_file");

#[derive(Debug, Failure)]
pub enum BooksError {
    #[failure(
        code = NOT_FOUND,
        recovery = Recovery::Run(Call::new(&["books", "list"])),
        field(book = *id),
    )]
    NotFound { id: u32 },
    #[failure(code = REMOVED_FILE, hint = "restore {name} from a backup")]
    RemovedFile { name: String },
}

impl fmt::Display for BooksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a books failure")
    }
}

impl std::error::Error for BooksError {}

let failure = BooksError::RemovedFile { name: "removed.txt".to_owned() };
assert_eq!(
    failure.recovery(),
    Recovery::Hint("restore removed.txt from a backup".to_owned())
);
assert_eq!(BooksError::CODES, &[&NOT_FOUND, &REMOVED_FILE]);
```

The type still implements `Display` and `std::error::Error` itself, by hand
or with another derive.

Each marked field of a row is a column, named as serde serializes it:

```rust
use crumbkit::Row;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Serialize, JsonSchema, Row)]
#[serde(rename_all = "camelCase")]
struct Book {
    #[row(column, uncut)]
    id: u32,
    #[row(column)]
    title: String,
    #[row(long)]
    page_count: u32,
}

assert_eq!(Book::COLUMNS, &["id", "title"]);
assert_eq!(Book::LONG_COLUMNS, &["pageCount"]);
```

The documentation of each derive in this crate lists its keys and shows
the code that it writes.

[crumbkit]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit/README.md
