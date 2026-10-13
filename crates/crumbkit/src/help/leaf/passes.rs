//! The declaration that a leaf's command passes on the exit code of a
//! program that it runs (ADR-0042).

use super::Leaf;

impl Leaf {
    /// Declares that the command passes on the exit code of `program`, a
    /// phrase that names the program it runs, such as `"the editor"`. The
    /// command's `-h` and `--help` then print `Exit code: the command passes
    /// on the exit code of the editor.` after the effects, and `<tool> help
    /// exit-codes` names the command (ADR-0042).
    ///
    /// crumbkit-test's `check_twin`, `check_twins` and `check_recoveries`
    /// then let a failure of the command that no exit lists end with any
    /// code but 0, and they still require a listed failure and a refused
    /// command line to exit as the guide declares (ADR-0037).
    #[must_use]
    pub const fn passes_on_exit(mut self, program: &'static str) -> Self {
        self.passes_on_exit = Some(program);
        self
    }

    /// Returns the program that [`Leaf::passes_on_exit`] set, or `None` when
    /// the command exits as the guide's exits declare. The name has clap's
    /// `get_` prefix, because `passes_on_exit` is the builder method.
    #[must_use]
    pub const fn get_passes_on_exit(&self) -> Option<&'static str> {
        self.passes_on_exit
    }
}
