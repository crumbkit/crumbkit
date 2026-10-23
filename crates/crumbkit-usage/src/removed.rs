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
//! `section`, which took any serializable value, stays removed, and a tool
//! records its own facts with `extend` (ADR-0144):
//!
//! ```compile_fail
//! let _ = crumbkit_usage::section::<()>;
//! ```
