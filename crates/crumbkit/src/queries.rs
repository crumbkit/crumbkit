//! The queries that a program which reads a tool runs: the command lines
//! that ask a crumbkit tool to describe itself and to report its usage log
//! (ADR-0200).
//!
//! A program such as flightdeck starts a tool it has never seen and reads
//! the tool's commands, documents and runs from what these queries print.
//! Each query runs one of crumbkit's commands at its fixed path with the
//! flags that make the result whole and machine-readable: `--json` prints a
//! JSON document, `--full` prints every field of every row, `--all` prints
//! every row of a list, and `--no-input` tells the tool that no person can
//! answer a question. [`QUERIES`] is the one list of them. The program builds
//! its argument vectors from the list, and crumbkit-test's `check_mounts`
//! parses each query against a tool's command tree, so a tool that would
//! refuse a query fails its own tests.
//!
//! ```
//! use crumbkit::queries::{COMMANDS_LIST, SCHEMAS_SHOW};
//!
//! let none: &[&str] = &[];
//! assert_eq!(
//!     COMMANDS_LIST.words(none),
//!     ["commands", "list", "--json", "--full", "--all", "--no-input"]
//! );
//! assert_eq!(
//!     SCHEMAS_SHOW.words(&["shelf/books-list/v1"]),
//!     ["schemas", "show", "--json", "--no-input", "--", "shelf/books-list/v1"]
//! );
//! ```
//!
//! The list leaves out two things that only the reading program knows. It
//! has no `--version` query, because clap provides that flag and it has no
//! path. It has no `--exclude-program` flag, because a program appends it to
//! the `self` queries only for a tool whose usage reports take it.

#[cfg(test)]
mod tests;

/// One query: the fixed path of one of crumbkit's commands, the flags that a
/// reading program passes to it, and whether document ids follow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Query {
    path: &'static [&'static str],
    flags: &'static [&'static str],
    takes_ids: bool,
}

impl Query {
    /// Returns the path of the query's command below the tool, such as
    /// `["schemas", "list"]`.
    #[must_use]
    pub const fn path(&self) -> &'static [&'static str] {
        self.path
    }

    /// Returns the flags that the query passes, each as a caller types it,
    /// such as `--json`.
    #[must_use]
    pub const fn flags(&self) -> &'static [&'static str] {
        self.flags
    }

    /// Reports whether the query takes ids, which follow `--` so that an id
    /// that starts with `-` stays an id. Only [`SCHEMAS_SHOW`] takes them.
    #[must_use]
    pub const fn takes_ids(&self) -> bool {
        self.takes_ids
    }

    /// Returns the query's argument vector after the program: the path, the
    /// flags and, for a query that takes ids, `--` and then `ids`. A query
    /// that takes no ids leaves `ids` out.
    #[must_use]
    pub fn words<'a, S: AsRef<str>>(&self, ids: &'a [S]) -> Vec<&'a str> {
        let mut words: Vec<&'a str> = self.path.iter().chain(self.flags).copied().collect();
        if self.takes_ids {
            words.push("--");
            words.extend(ids.iter().map(AsRef::as_ref));
        }
        words
    }
}

/// The flags of a query that reads every row of a list.
const LIST_FLAGS: &[&str] = &["--json", "--full", "--all", "--no-input"];

/// `commands list --json --full --all --no-input`, which prints every
/// command with its arguments, examples and columns.
pub const COMMANDS_LIST: Query = Query {
    path: &["commands", "list"],
    flags: LIST_FLAGS,
    takes_ids: false,
};

/// `schemas list --json --full --all --no-input`, which prints every
/// document and the commands that print it.
pub const SCHEMAS_LIST: Query = Query {
    path: &["schemas", "list"],
    flags: LIST_FLAGS,
    takes_ids: false,
};

/// `schemas show --json --no-input -- <ID>...`, which prints the JSON Schema
/// of each named document. The command takes at most 100 ids in one call.
pub const SCHEMAS_SHOW: Query = Query {
    path: &["schemas", "show"],
    flags: &["--json", "--no-input"],
    takes_ids: true,
};

/// `self figures list --json --full --all --no-input`, which prints the
/// figures of each command's runs from the tool's usage log.
pub const SELF_FIGURES_LIST: Query = Query {
    path: &["self", "figures", "list"],
    flags: LIST_FLAGS,
    takes_ids: false,
};

/// `self usage --json --full --no-input`, which prints the patterns in the
/// tool's usage log where callers waste effort.
pub const SELF_USAGE: Query = Query {
    path: &["self", "usage"],
    flags: &["--json", "--full", "--no-input"],
    takes_ids: false,
};

/// `self runs list --json --full --all --no-input`, which prints each run in
/// the tool's usage log in order within its session.
pub const SELF_RUNS_LIST: Query = Query {
    path: &["self", "runs", "list"],
    flags: LIST_FLAGS,
    takes_ids: false,
};

/// Every query, in the order a reading program runs them: the tool's
/// commands, its documents and their schemas, and then its usage log.
pub const QUERIES: &[Query] = &[
    COMMANDS_LIST,
    SCHEMAS_LIST,
    SCHEMAS_SHOW,
    SELF_FIGURES_LIST,
    SELF_USAGE,
    SELF_RUNS_LIST,
];
