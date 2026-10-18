//! Doctests that fail to compile while an item that ADR-0085 removed is
//! absent. No tool and no example called these items. rustdoc compiles each
//! block below and passes only when the block fails to compile.
//!
//! The examples redact no value that the tool names:
//!
//! ```compile_fail
//! let _ = crumbkit_test::Examples::new("shelf", "golden").redact(&[("[ROOT]", "/tmp")]);
//! ```
//!
//! The recovery check, the twin check and an unprovoked code take codes by
//! reference only:
//!
//! ```compile_fail
//! use crumbkit::{Code, Spelling};
//! use crumbkit_test::{Examples, Recoveries};
//!
//! let codes = [Code::new("shelf", "books", "not_found")];
//! let _ = Recoveries::from_codes(Examples::new("shelf", ""), Spelling::COLONS, &codes);
//! ```
//!
//! ```compile_fail
//! use crumbkit::{Code, Spelling};
//! use crumbkit_test::{Examples, Twins};
//!
//! let codes = [Code::new("shelf", "books", "not_found")];
//! let _ = Twins::from_codes(Examples::new("shelf", ""), Spelling::COLONS, &codes);
//! ```
//!
//! ```compile_fail
//! use crumbkit::Code;
//! use crumbkit_test::Unprovoked;
//!
//! let _ = Unprovoked::from_code(Code::new("shelf", "books", "gone"), "A broken disk causes it.");
//! ```
