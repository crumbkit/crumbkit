//! The guide's declaration that a command named in another style also
//! answers to its dashed form (ADR-0015).

use super::Guide;

impl Guide {
    /// Declares that each command named in another style, such as
    /// `reading_lists` or `ReadingLists`, also answers to its
    /// [`dashed_form`](crate::dashed_form), such as `reading-lists`.
    ///
    /// With the declaration, [`parse`](crate::parse()), completion and
    /// [`command_path`](crate::command_path) read a typed dashed form as the
    /// command, and help, completion candidates, the usage log and the
    /// parsed words name the command by the tool's own name only. Without
    /// it, a dashed form names no command, and clap refuses it as it
    /// refuses any unknown word. crumbkit-test's `check_grammar` then
    /// accepts such a name without an opt-out, and `check_opt_outs` lists
    /// the declaration and each such command as customisations (ADR-0056).
    ///
    /// ```
    /// use crumbkit::help::Guide;
    ///
    /// const GUIDE: Guide = Guide::new().dashed_forms();
    /// assert!(GUIDE.get_dashed_forms());
    /// assert!(!Guide::new().get_dashed_forms());
    /// ```
    #[must_use]
    pub const fn dashed_forms(mut self) -> Self {
        self.dashed_forms = true;
        self
    }

    /// Reports whether the tool declared [`Guide::dashed_forms`]. The name
    /// has clap's `get_` prefix.
    #[must_use]
    pub const fn get_dashed_forms(&self) -> bool {
        self.dashed_forms
    }
}
