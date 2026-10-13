# crumbkit-derive

crumbkit-derive holds `#[derive(Failure)]`, which writes the `Failure` impl
of [crumbkit] for an enum or a struct from `#[failure(...)]` attributes. It
also writes a `CODES` constant that lists the type's codes for the codes
check. A tool does not add this crate itself. It turns on the `derive`
feature of crumbkit or crumbkit-core, which re-exports the derive beside the
trait of the same name. crumbkit is in design and is not published yet.

## Adding it

```toml
[dependencies]
crumbkit = { git = "https://github.com/crumbkit/crumbkit", features = ["derive"] }
```

The derive builds syn, quote and proc-macro2, so the feature is off by
default. A tool that writes its impls by hand builds none of them.

## Example

Each variant names its code and its next step:

```rust
# extern crate crumbkit_core as crumbkit;
use std::fmt;

use crumbkit::{Call, Code, Failure, Recovery};

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const REMOVED_FILE: Code = Code::new("shelf", "books", "removed_file");

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
or with another derive. The documentation of `Failure` in this crate lists
every key and shows the code that the derive writes.

[crumbkit]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit/README.md
