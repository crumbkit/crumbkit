//! Doctests that fail to compile while an item that ADR-0085 removed is
//! absent. No tool and no example called these items, and ADR-0088 and
//! ADR-0089 restored the ones that a tool outside this repository calls.
//! rustdoc compiles each block below and passes only when the block fails to
//! compile.
//!
//! The examples redact no value that the tool names:
//!
//! ```compile_fail
//! let _ = crumbkit_test::Examples::new("shelf", "golden").redact(&[("[ROOT]", "/tmp")]);
//! ```
