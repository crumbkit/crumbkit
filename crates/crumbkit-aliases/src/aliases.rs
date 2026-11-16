//! The ordered sources that a tool's aliases come from (ADR-0228).

use crate::source::Source;

/// The aliases of a tool: an ordered list of [`Source`]s.
///
/// The first source that has a matching entry wins, so a tool lists the
/// person's file first and its own sources after it. A tool builds one
/// value, usually in a `static`, and calls [`expand`](Aliases::expand)
/// before parsing, [`names`](Aliases::names) for the root's help and
/// [`list`](Aliases::list) for its hidden `aliases list` command:
///
/// ```
/// use crumbkit_aliases::{Aliases, Source};
///
/// let aliases = Aliases::new()
///     .source(Source::file(crumbkit_aliases::file("shelf")))
///     .source(Source::table("shelf", &[("wt ls", &["books", "list"])]));
/// ```
#[derive(Clone, Debug, Default)]
pub struct Aliases {
    /// The sources, in precedence order.
    sources: Vec<Source>,
}

impl Aliases {
    /// Returns aliases with no source.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Returns these aliases with `source` after the sources added so far,
    /// so an earlier source wins when two match.
    #[must_use]
    pub fn source(mut self, source: Source) -> Self {
        self.sources.push(source);
        self
    }

    /// Returns the sources, in precedence order.
    pub(crate) fn sources(&self) -> &[Source] {
        &self.sources
    }
}
