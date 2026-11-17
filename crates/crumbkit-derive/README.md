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
assert_eq!(BooksError::CODES, &[NOT_FOUND, REMOVED_FILE]);
```

## Reviewed sentences

A variant can carry a fixed sentence that a person reviewed, which is safe to
show to anyone. `#[failure(reviewed)]` on the enum makes a variant without a
`sentence` a compile error that names the variant. The sentence is a string
literal on one line with no `{`, so no value from the run reaches it. A
variant that uses `forward` takes its sentence from the failure it wraps. The derive does not check that the wrapped type has one, so a forwarded type is checked only when it is marked `reviewed` itself.

```rust
use std::fmt;

use crumbkit::{Call, Code, Failure, Recovery};

const NOT_FOUND: Code<'static> = Code::new("shelf", "books", "not_found");
const NO_STATE: Code<'static> = Code::new("shelf", "state", "missing");

#[derive(Debug, Failure)]
#[failure(reviewed)]
pub enum ReviewedError {
    #[failure(
        code = NOT_FOUND,
        sentence = "The catalogue has no book with that id.",
        recovery = Recovery::Run(Call::new(&["books", "list"])),
    )]
    NotFound,
    #[failure(
        code = NO_STATE,
        sentence = "The state directory is missing.",
        hint = "set HOME to a directory",
    )]
    NoState,
}

impl fmt::Display for ReviewedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a books failure")
    }
}

impl std::error::Error for ReviewedError {}

assert_eq!(
    ReviewedError::NoState.sentence(),
    Some("The state directory is missing.")
);
```

A variant without a sentence under `reviewed` does not compile:

```compile_fail
use std::fmt;

use crumbkit::{Code, Failure};

const NO_STATE: Code<'static> = Code::new("shelf", "state", "missing");

#[derive(Debug, Failure)]
#[failure(reviewed)]
pub enum ReviewedError {
    #[failure(code = NO_STATE, hint = "set HOME to a directory")]
    NoState,
}

impl fmt::Display for ReviewedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a books failure")
    }
}

impl std::error::Error for ReviewedError {}
```

The check covers a type that derives `Failure`. A failure written by hand and
a code from `Code::runtime` escape it.

A `code` can be an expression that reads the variant's fields, with
`codes = [..]` naming what it returns. A foreign error is forwarded through
a newtype that implements `Failure`.

A struct that derives `Row` marks its column fields, and the same attribute
can name the child lines, a problem decided by an expression, the columns of a
flattened field and a mirror of a foreign type; the documentation of the
derive lists them.

The type still implements `Display` and `std::error::Error` itself, by hand
or with another derive. The documentation of each derive in this crate
lists its keys and shows the code that it writes.

[crumbkit]: https://github.com/crumbkit/crumbkit/blob/main/crates/crumbkit/README.md
