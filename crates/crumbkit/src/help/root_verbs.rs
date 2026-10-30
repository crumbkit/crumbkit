//! The guide's declaration of the verbs that sit directly under the tool's
//! root, which root help lists under their own heading (ADR-0165).

use super::Guide;

impl Guide {
    /// Declares `verbs`, the names of the commands without subcommands that
    /// sit directly under the root on purpose, such as `start` and `stop`,
    /// with the tool's `reason` for the whole list.
    ///
    /// The root's help, from [`parse`](crate::parse()) and
    /// [`extend`](super::extend), lists each of `verbs` that no goal of a
    /// root [`Group`](super::Group) names under the heading `Verbs`, in the
    /// order of `verbs`, after the root group's goals and before the
    /// `Commands` heading of the other subcommands. The root is the clap
    /// root, the command that the tool's own name runs. A guide that
    /// declares no root verbs keeps the root's help as it was.
    ///
    /// crumbkit-test's `check_grammar` then passes each of `verbs` without
    /// an opt-out from its root-leaf rule, and `check_opt_outs` lists the
    /// declaration and `reason` among the customisations.
    ///
    /// ```
    /// use crumbkit::help::Guide;
    ///
    /// const GUIDE: Guide = Guide::new().root_verbs(
    ///     &["start", "stop"],
    ///     "People and scripts type `start` and `stop` as one word every day.",
    /// );
    /// assert_eq!(GUIDE.get_root_verbs().map(|(verbs, _)| verbs.len()), Some(2));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A
    /// `const` that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit::help::Guide;
    ///
    /// const BAD: Guide = Guide::new().root_verbs(&["start"], "");
    /// ```
    #[must_use]
    pub const fn root_verbs(
        mut self,
        verbs: &'static [&'static str],
        reason: &'static str,
    ) -> Self {
        assert!(
            crate::document::is_one_line(reason),
            "a root verbs reason is one non-empty line"
        );
        self.root_verbs = Some((verbs, reason));
        self
    }

    /// Returns the verbs and the reason that [`Guide::root_verbs`] set, or
    /// `None` when the tool declares no root verbs. The name has clap's
    /// `get_` prefix.
    #[must_use]
    pub const fn get_root_verbs(&self) -> Option<(&'static [&'static str], &'static str)> {
        self.root_verbs
    }

    /// Returns the verbs that [`Guide::root_verbs`] set, or none.
    pub(crate) const fn root_verb_names(&self) -> &'static [&'static str] {
        match self.root_verbs {
            Some((verbs, _)) => verbs,
            None => &[],
        }
    }
}
