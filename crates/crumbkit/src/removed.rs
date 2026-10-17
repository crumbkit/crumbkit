//! Doctests that fail to compile while an option that ADR-0081 removed is
//! absent. Each option reproduced the output shape of one adopting tool, and
//! API-1 asks for a second use before such an option returns. rustdoc
//! compiles each block below and passes only when the block fails to
//! compile.
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
//! A tool cannot implement a failure form of its own:
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
