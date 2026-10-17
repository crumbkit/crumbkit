//! The tool's own codes for crumbkit's failures, and the exit code that a
//! failure ends a run with (ADR-0056).

use crate::Code;
use crate::help::{Exit, Guide, Leaf};

#[cfg(test)]
mod tests;

impl Guide {
    /// Sets the tool's own codes for crumbkit's failures. A run prints the
    /// tool's code of each [`Rename`] in place of crumbkit's, and the usage
    /// log, the trace and crumbkit-test's checks read the same code. A
    /// tool that renames nothing prints crumbkit's codes (ADR-0056).
    #[must_use]
    pub const fn codes(mut self, renames: &'static [Rename]) -> Self {
        self.codes = renames;
        self
    }

    /// Returns the renames that [`Guide::codes`] set. The name has clap's
    /// `get_` prefix, because `codes` is the builder method.
    #[must_use]
    pub const fn get_codes(&self) -> &'static [Rename] {
        self.codes
    }

    /// Returns the code that a run of the command at `path`, the real names
    /// of its subcommands from the root, prints for a failure with `code`.
    /// It is the tool's code when the guide's leaf at `path` renames `code`
    /// with [`Leaf::codes`], then the tool's code when [`Guide::codes`]
    /// renames it, and `code` otherwise (ADR-0060).
    #[must_use]
    pub fn printed<'code>(&self, path: &[&str], code: &'code Code) -> &'code Code {
        Renames::at(self, path).printed(code)
    }

    /// Returns the exit code of a run of the command at `path` whose first
    /// failure has `code`. It is the code of the first [`Exit`] that lists
    /// the failure's printed code or a crumbkit code that the run prints as
    /// it, where [`Guide::printed`] decides the printed code for `path`. A
    /// failure that no exit lists exits with crumbkit's default: 2 for a
    /// refused command line and 1 for every other failure (ADR-0002).
    #[must_use]
    pub fn failure_exit(&self, path: &[&str], code: &'static Code) -> u8 {
        failure_exit(self.exits, Renames::at(self, path), code)
    }
}

/// One of crumbkit's failure codes and the tool's own code that crumbkit
/// prints in its place, such as `ledger::cli::usage` for
/// `crumbkit::cli::usage`.
///
/// A tool lists its renames in its [`Guide`] with [`Guide::codes`], and
/// the renames of one command in its leaf with [`Leaf::codes`], which a run
/// of that command reads before the guide's. crumbkit still decides when each
/// of its failures happens, with its message, its recovery and its
/// alternatives, and the rename changes only the code that a run prints,
/// the code that the usage log and the trace record, and the exit that an
/// [`Exit`] lists for it:
///
/// ```
/// use crumbkit::Code;
/// use crumbkit::codes::CLI_USAGE;
/// use crumbkit::help::{Exit, Guide, Rename};
///
/// const USAGE: Code = Code::new("ledger", "cli", "usage");
///
/// const GUIDE: Guide = Guide::new()
///     .codes(&[Rename::new(&CLI_USAGE, &USAGE)])
///     .exits(&[Exit::new(64, "The command line was refused.").failures(&[&USAGE])]);
///
/// assert_eq!(GUIDE.printed(&[], &CLI_USAGE), &USAGE);
/// assert_eq!(GUIDE.failure_exit(&[], &CLI_USAGE), 64);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rename {
    /// The code that crumbkit's failure has.
    crumbkit: &'static Code,
    /// The tool's code, which crumbkit prints in its place.
    tool: &'static Code,
}

impl Rename {
    /// Returns the rename that prints `tool` in place of `crumbkit`.
    ///
    /// # Panics
    ///
    /// Panics unless `crumbkit` is in the `crumbkit` component and `tool`
    /// is not, because a rename gives one of crumbkit's codes the tool's
    /// name. A `const` that calls it with such codes fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit::Code;
    /// use crumbkit::help::Rename;
    ///
    /// const MINE: Code = Code::new("ledger", "rows", "stale");
    /// const BACKWARDS: Rename = Rename::new(&MINE, &crumbkit::codes::VERSION_CONFLICT);
    /// ```
    #[must_use]
    pub const fn new(crumbkit: &'static Code, tool: &'static Code) -> Self {
        assert!(
            same_text(crumbkit.component(), "crumbkit"),
            "a rename's first code is one of crumbkit's codes"
        );
        assert!(
            !same_text(tool.component(), "crumbkit"),
            "a rename's second code is the tool's own code, outside the crumbkit component"
        );
        Self { crumbkit, tool }
    }

    /// Returns crumbkit's code.
    #[must_use]
    pub const fn crumbkit(&self) -> &'static Code {
        self.crumbkit
    }

    /// Returns the tool's code, which crumbkit prints in place of
    /// [`Rename::crumbkit`].
    #[must_use]
    pub const fn tool(&self) -> &'static Code {
        self.tool
    }
}

/// Returns whether `left` and `right` hold the same text. It compares bytes
/// because `str` equality is not `const`.
pub(crate) const fn same_text(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// The renames that one run reads: the renames of the leaf of the run's
/// command, which come first, and the guide's (ADR-0060).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Renames {
    /// The renames that [`Leaf::codes`] set on the leaf of the run's
    /// command, or none.
    leaf: &'static [Rename],
    /// The renames that [`Guide::codes`] set.
    guide: &'static [Rename],
}

impl Renames {
    /// No renames, as a run without a guide reads them.
    pub(crate) const NONE: Self = Self {
        leaf: &[],
        guide: &[],
    };

    /// Returns the renames of `guide` alone, which a run reads before it
    /// knows its command, such as for a refused command line.
    pub(crate) const fn of(guide: &Guide) -> Self {
        Self {
            leaf: &[],
            guide: guide.get_codes(),
        }
    }

    /// Returns the renames of `leaf`, or none, and of `guide`.
    pub(crate) const fn with_leaf(guide: &Guide, leaf: Option<&Leaf>) -> Self {
        let leaf = match leaf {
            Some(leaf) => leaf.get_codes(),
            None => &[],
        };
        Self {
            leaf,
            guide: guide.get_codes(),
        }
    }

    /// Returns the renames that a run of the command at `path` reads.
    pub(crate) fn at(guide: &Guide, path: &[&str]) -> Self {
        let leaf = guide.get_leaves().iter().find(|leaf| leaf.path() == path);
        Self::with_leaf(guide, leaf)
    }

    /// Returns the code that a run prints for a failure with `code`: the
    /// tool's code of the leaf's rename of `code`, then of the guide's, and
    /// `code` when neither renames it.
    pub(crate) fn printed(self, code: &Code) -> &Code {
        self.leaf
            .iter()
            .chain(self.guide)
            .find(|rename| rename.crumbkit == code)
            .map_or(code, |rename| rename.tool)
    }
}

/// Returns the exit code of a run whose first failure has `code`: the code
/// of the first of `exits` that lists the failure's printed code, or one of
/// crumbkit's codes that `renames` prints as it. A failure that no exit
/// lists exits with crumbkit's default: 2 for a refused command line and 1
/// for every other failure (ADR-0002).
pub(crate) fn failure_exit(exits: &[Exit], renames: Renames, code: &'static Code) -> u8 {
    if let Some(exit) = listed_exit(exits, renames, code) {
        return exit;
    }
    // A leaf cannot rename `crumbkit::cli::usage`, so the guide's renames
    // decide the refusal's printed code for every command.
    if renames.printed(code) == renames.printed(&crate::codes::CLI_USAGE) {
        return 2;
    }
    1
}

/// Returns the code of the first of `exits` that lists the printed code of
/// a failure with `code`, or one of crumbkit's codes that `renames` prints
/// as it, and `None` when no exit lists it.
pub(crate) fn listed_exit(exits: &[Exit], renames: Renames, code: &'static Code) -> Option<u8> {
    let code = renames.printed(code);
    exits
        .iter()
        .find(|exit| {
            exit.get_failures()
                .iter()
                .any(|&listed| renames.printed(listed) == code)
        })
        .map(Exit::code)
}
