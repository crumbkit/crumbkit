//! Doctests that fail to compile while an item that ADR-0095 removed is
//! absent. No tool in this repository called these items. rustdoc compiles
//! each block below and passes only when the block fails to compile.
//!
//! The log's thread always handles SIGINT and SIGTERM:
//!
//! ```compile_fail
//! let _ = crumbkit_usage::start_with;
//! ```
//!
//! ```compile_fail
//! let _ = crumbkit_usage::Signals::Tool;
//! ```
//!
//! The line holds no section of the tool's own:
//!
//! ```compile_fail
//! let _ = crumbkit_usage::section::<()>;
//! ```
