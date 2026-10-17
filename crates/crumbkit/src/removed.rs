//! Doctests that fail to compile while an item that ADR-0081 or ADR-0085
//! removed is absent. Each option that ADR-0081 removed reproduced the
//! output shape of one adopting tool, and API-1 asks for a second use before
//! such an option returns. ADR-0085 removed items that no tool or sibling
//! crate called. rustdoc compiles each block below and passes only when the
//! block fails to compile.
//!
//! A document cannot print its id and result under keys that the tool
//! declares:
//!
//! ```compile_fail
//! use crumbkit::{Document, Envelope};
//!
//! const ROWS: Document<()> = Document::new("tool", "get", 1, "Rows.")
//!     .envelope(Envelope::Keys { type_key: "$type", data_key: "rows" });
//! ```
//!
//! A batch with declared keys cannot print its failures among its rows in
//! input order:
//!
//! ```compile_fail
//! let _ = crumbkit::Envelope::KeysInOrder { type_key: "$type", data_key: "rows" };
//! ```
//!
//! A batch without an envelope cannot print its failures among its rows in
//! input order:
//!
//! ```compile_fail
//! let _ = crumbkit::Envelope::WithoutInOrder;
//! ```
//!
//! A guide cannot list failure field names that break crumbkit's rule:
//!
//! ```compile_fail
//! use crumbkit::help::Guide;
//!
//! const GUIDE: Guide = Guide::new().field_names(&["requestId"]);
//! ```
//!
//! A tool cannot implement a failure form of its own. ADR-0085 also removed
//! the whole `form` module, so the path below no longer resolves at all:
//!
//! ```compile_fail
//! use crumbkit::form::FailureForm;
//! ```
//!
//! A guide holds no failure form:
//!
//! ```compile_fail
//! let _ = crumbkit::help::Guide::new().get_failure_form();
//! ```
//!
//! A guide cannot sort the keys of every JSON object:
//!
//! ```compile_fail
//! let _ = crumbkit::JsonStyle::new().sorted();
//! ```
//!
//! A guide sets the layout of its JSON without a separate style value:
//!
//! ```compile_fail
//! let _ = crumbkit::help::Guide::new().get_json();
//! ```
//!
//! A batch cannot answer each key that the output budget leaves out with a
//! failure of its own:
//!
//! ```compile_fail
//! use crumbkit::{Batch, Document, Error, Row};
//!
//! #[derive(serde::Serialize, schemars::JsonSchema)]
//! struct Book {
//!     id: u32,
//! }
//!
//! impl Row for Book {
//!     const COLUMNS: &'static [&'static str] = &["id"];
//! }
//!
//! const BOOKS_SHOW: Document<Batch<Book, Error>> =
//!     Document::new("shelf", "books-show", 1, "The books.").left_out_per_key();
//! ```
//!
//! A tool cannot make a failure code from segments it learns at run time:
//!
//! ```compile_fail
//! let _ = crumbkit::Code::runtime("plugin", "sync", "timed_out");
//! ```
//!
//! crumbkit has no error type for a run-time code:
//!
//! ```compile_fail
//! let _: Option<crumbkit::RuntimeCodeError> = None;
//! ```
//!
//! A page does not compute the number of rows to fetch:
//!
//! ```compile_fail
//! fn rows(page: &crumbkit::Page) -> Option<usize> {
//!     page.fetch_limit()
//! }
//! ```
//!
//! A stopped run gives its exit code as an `ExitCode` only:
//!
//! ```compile_fail
//! fn number(stop: &crumbkit::Stop) -> u8 {
//!     stop.number()
//! }
//! ```
//!
//! `--expect` is read through `Expect::check` only:
//!
//! ```compile_fail
//! fn expected(flag: &crumbkit::Expect) -> bool {
//!     flag.get().is_some()
//! }
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit::Expected::Nothing;
//! ```
//!
//! The schema of one line of a stream is served by `schemas show` only:
//!
//! ```compile_fail
//! use crumbkit::{Document, Row, Stream};
//!
//! #[derive(serde::Serialize, schemars::JsonSchema)]
//! struct Entry {
//!     id: u32,
//! }
//!
//! impl Row for Entry {
//!     const COLUMNS: &'static [&'static str] = &["id"];
//! }
//!
//! const ENTRIES: Document<Stream<Entry>> = Document::new("tool", "entries", 1, "Entries.");
//!
//! let _ = crumbkit::schemas::stream_schema(ENTRIES);
//! ```
//!
//! A leaf does not give back its `Output:` line. A renderer of help prints
//! `help::STREAM_LINE`, `help::BYTES_LINE` or `help::HEADED_LINE`:
//!
//! ```compile_fail
//! let _ = crumbkit::help::Leaf::new(&["books", "list"]).output_line();
//! ```
//!
//! A guide does not give back the file formats it serves:
//!
//! ```compile_fail
//! let _ = crumbkit::help::Guide::new().get_documents();
//! ```
//!
//! Effects answer whether a command is read-only, destructive or replacing,
//! and not whether it is idempotent or open-world:
//!
//! ```compile_fail
//! let _ = crumbkit::help::Effects::read_only().is_idempotent();
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit::help::Effects::read_only().is_open_world();
//! ```
//!
//! A tool spells its codes with colons or with `Spelling::custom`:
//!
//! ```compile_fail
//! let _ = crumbkit::Spelling::DOTS;
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit::Spelling::SLASHES;
//! ```
//!
//! A spelling does not read a spelled code back into its segments:
//!
//! ```compile_fail
//! let _ = crumbkit::Spelling::COLONS.split("shelf::books::not_found");
//! ```
//!
//! The failure that crumbkit-test reads back from a run is private to
//! crumbkit-test:
//!
//! ```compile_fail
//! let _ = crumbkit::form::ReadFailure::new(String::new(), Vec::new());
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit::form::Step::Retry;
//! ```
//!
//! A tool reads the bytes of a result through `Bytes::as_slice`:
//!
//! ```compile_fail
//! let _: Vec<u8> = crumbkit::Bytes::new(vec![1]).into_vec();
//! ```
