//! The tool's own codes for crumbkit's failures, and the exit code that a
//! failure ends a run with (ADR-0056).

use crate::Code;
use crate::help::{Exit, Guide};

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

    /// Returns the code that a run prints for a failure with `code`: the
    /// tool's code when [`Guide::codes`] renames `code`, and `code`
    /// otherwise.
    #[must_use]
    pub fn printed<'code>(&self, code: &'code Code) -> &'code Code {
        printed(self.codes, code)
    }

    /// Returns the exit code of a run whose first failure has `code`. It is
    /// the code of the first [`Exit`] that lists the failure's printed code
    /// or a crumbkit code that the guide prints as it. A failure that no
    /// exit lists exits with crumbkit's default: 2 for a refused command
    /// line, the same code as a refused command line for a child that
    /// cannot answer `--json`, and 1 for every other failure (ADR-0002).
    #[must_use]
    pub fn failure_exit(&self, code: &'static Code) -> u8 {
        failure_exit(self.exits, self.codes, code)
    }
}

/// One of crumbkit's failure codes and the tool's own code that crumbkit
/// prints in its place, such as `ledger::cli::usage` for
/// `crumbkit::cli::usage`.
///
/// A tool lists its renames in its [`Guide`] with
/// [`Guide::codes`]. crumbkit still decides when each
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
/// assert_eq!(GUIDE.printed(&CLI_USAGE), &USAGE);
/// assert_eq!(GUIDE.failure_exit(&CLI_USAGE), 64);
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
            is_crumbkit(crumbkit),
            "a rename's first code is one of crumbkit's codes"
        );
        assert!(
            !is_crumbkit(tool),
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

/// Returns whether `code` is in the `crumbkit` component. It compares bytes
/// because `str` equality is not `const`.
const fn is_crumbkit(code: &Code) -> bool {
    let component = code.component().as_bytes();
    let crumbkit = b"crumbkit";
    if component.len() != crumbkit.len() {
        return false;
    }
    let mut index = 0;
    while index < component.len() {
        if component[index] != crumbkit[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// Returns the code that a run prints for a failure with `code`: the tool's
/// code when `renames` renames it, and `code` otherwise.
pub(crate) fn printed<'code>(renames: &[Rename], code: &'code Code) -> &'code Code {
    renames
        .iter()
        .find(|rename| rename.crumbkit == code)
        .map_or(code, |rename| rename.tool)
}

/// Returns the exit code of a run whose first failure has `code`: the code
/// of the first of `exits` that lists the failure's printed code, or one of
/// crumbkit's codes that `renames` prints as it. A failure that no exit
/// lists exits with crumbkit's default: 2 for a refused command line, the
/// refusal's exit for a child that cannot answer `--json`, and 1 for every
/// other failure (ADR-0002).
pub(crate) fn failure_exit(exits: &[Exit], renames: &[Rename], code: &'static Code) -> u8 {
    let code = printed(renames, code);
    let listed = exits.iter().find(|exit| {
        exit.get_failures()
            .iter()
            .any(|&listed| printed(renames, listed) == code)
    });
    if let Some(exit) = listed {
        return exit.code();
    }
    if code == printed(renames, &crate::codes::CLI_USAGE) {
        return 2;
    }
    #[cfg(feature = "hand-over")]
    if code == printed(renames, &crate::codes::CHILD_NO_JSON) {
        return failure_exit(exits, renames, &crate::codes::CLI_USAGE);
    }
    1
}
