//! The tool's own codes for crumbkit's failures in the runs of one command
//! (ADR-0060).

use super::Leaf;
use crate::help::Rename;
use crate::help::rename::same_text;

impl Leaf {
    /// Sets the tool's own codes for crumbkit's failures in the runs of the
    /// command. A run of the command prints the tool's code of each
    /// [`Rename`] in place of crumbkit's, and reads the renames of
    /// [`Guide::codes`](crate::help::Guide::codes) for the codes that the
    /// leaf does not rename. The usage log, the trace, the exit that an
    /// [`Exit`](crate::help::Exit) lists for a code and crumbkit-test's
    /// checks read the same code (ADR-0060).
    ///
    /// ```
    /// use crumbkit::Code;
    /// use crumbkit::codes::VERSION_CONFLICT;
    /// use crumbkit::help::{Guide, Leaf, Rename};
    ///
    /// const MOVED: Code = Code::new("ledger", "rows", "moved");
    ///
    /// const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["rows", "move"])
    ///     .codes(&[Rename::new(&VERSION_CONFLICT, &MOVED)])]);
    ///
    /// assert_eq!(GUIDE.printed(&["rows", "move"], &VERSION_CONFLICT), &MOVED);
    /// assert_eq!(GUIDE.printed(&["rows", "list"], &VERSION_CONFLICT), &VERSION_CONFLICT);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when a rename names `<tool>::cli::usage`, because crumbkit
    /// refuses a command line before it knows the command, so only
    /// [`Guide::codes`](crate::help::Guide::codes) renames that code. A
    /// `const` that calls it with such a rename fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit::Code;
    /// use crumbkit::codes::CLI_USAGE;
    /// use crumbkit::help::{Leaf, Rename};
    ///
    /// const BAD_LINE: Code = Code::new("ledger", "cli", "bad_line");
    /// const LEAF: Leaf = Leaf::new(&["rows", "move"]).codes(&[Rename::new(&CLI_USAGE, &BAD_LINE)]);
    /// ```
    #[must_use]
    pub const fn codes(mut self, renames: &'static [Rename]) -> Self {
        let mut index = 0;
        while index < renames.len() {
            let code = renames[index].crumbkit();
            assert!(
                !(same_text(code.area(), "cli") && same_text(code.reason(), "usage")),
                "a leaf cannot rename crumbkit::cli::usage, which only Guide::codes renames"
            );
            index += 1;
        }
        self.codes = renames;
        self
    }

    /// Returns the renames that [`Leaf::codes`] set. The name has clap's
    /// `get_` prefix, because `codes` is the builder method.
    #[must_use]
    pub const fn get_codes(&self) -> &'static [Rename] {
        self.codes
    }
}
