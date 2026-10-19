//! Doctests that fail to compile while an item that ADR-0095 removed is
//! absent. No tool in this repository called these items. rustdoc compiles
//! each block below and passes only when the block fails to compile.
//!
//! The crate hands over no child that prints a JSON document:
//!
//! ```compile_fail
//! let _ = crumbkit_child::hand_over_json;
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit_child::Json::<()>::new;
//! ```
//!
//! The crate declares no code for a refused `--json` or for a child's output
//! that is not a document:
//!
//! ```compile_fail
//! let _ = crumbkit_child::codes::NO_JSON;
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit_child::codes::BAD_JSON;
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit_child::codes::FAILED;
//! ```
