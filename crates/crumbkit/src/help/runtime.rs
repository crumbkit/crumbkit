//! Runtime leaves, which give examples to the commands that a tool adds to
//! its command tree at run time, such as one command for each file that a
//! person writes.

use super::{Example, Guide, Leaf};

#[cfg(test)]
mod tests;

/// A leaf that a tool declares at run time, for a command that it adds to
/// its command tree at run time. Its help lists the examples, as the help
/// of a [`Leaf`] in a `const` [`Guide`] does.
///
/// A tool builds one for each command it adds, from the same data, and
/// joins them to its guide with [`Guide::with_runtime_leaves`]. It then
/// passes the joined guide to [`parse`](crate::parse),
/// [`commands::list`](crate::commands::list) and crumbkit-test's checks:
///
/// ```
/// use crumbkit::help::{Guide, RuntimeLeaf};
///
/// const GUIDE: Guide = Guide::new();
///
/// let name = String::from("classics");
/// let guide = GUIDE.with_runtime_leaves(vec![
///     RuntimeLeaf::new(vec![name.clone(), "list".to_owned()])
///         .example(Vec::new(), format!("List the entries of {name}")),
/// ]);
///
/// assert_eq!(guide.get_leaves()[0].path(), ["classics", "list"]);
/// ```
///
/// crumbkit's example tool `reading`, in [`examples/reading`], adds a
/// command for each reading list in a directory and builds its guide this
/// way. Its tests pass the tree and the guide of a fixture directory to
/// every crumbkit-test check.
///
/// [`examples/reading`]: https://github.com/crumbkit/crumbkit/tree/main/examples/reading
///
/// A runtime leaf has examples and no document, so its `--help` lists no
/// columns. A document's name is the path of the command that prints it
/// (ADR-0035), and a path chosen at run time names no document that the
/// tool declares as a constant. A command added at run time prints a
/// document that a leaf of the `const` guide declares.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeLeaf {
    /// The names of the subcommands from the root to the command.
    path: Vec<String>,
    /// The examples, each the words after the path and a description, in
    /// the order the help page lists them.
    examples: Vec<(Vec<String>, String)>,
}

impl RuntimeLeaf {
    /// Returns the leaf of the command at `path`, the real names of its
    /// subcommands from the root, as [`Leaf::new`] names a command. The
    /// leaf has no examples.
    #[must_use]
    pub const fn new(path: Vec<String>) -> Self {
        Self {
            path,
            examples: Vec::new(),
        }
    }

    /// Adds the example that runs the command with `words`, the arguments
    /// after its path, described by `about`, as [`Example::new`] describes
    /// an example. The help page lists the examples in the order they were
    /// added.
    ///
    /// The method is not a `const fn`, because it adds to a `Vec`.
    #[must_use]
    pub fn example(mut self, words: Vec<String>, about: String) -> Self {
        self.examples.push((words, about));
        self
    }
}

impl Guide {
    /// Returns a guide with the groups and topics of `self`, and the leaves
    /// of `self` followed by `leaves`, each with its path and examples.
    ///
    /// Every function that reads a guide reads the joined one as it reads a
    /// `const` guide, and of two leaves with one path only the first
    /// applies, so a runtime leaf with the path of a `const` leaf changes
    /// nothing. crumbkit-test's `check_guide` reports both.
    ///
    /// A tool without commands added at run time does not call it, and its
    /// runs do no work for runtime leaves.
    //
    // RUST-10: the joined guide holds `'static` slices, as a `const` guide
    // does, so this leaks the runtime leaves' strings and the joined list of
    // leaves for the rest of the process. A run calls it once, and the leak
    // is the size of the runtime leaves. A guide that borrows its runtime
    // leaves, with a lifetime on `Guide`, replaces this if a process needs
    // to build many guides.
    #[must_use]
    pub fn with_runtime_leaves(self, leaves: Vec<RuntimeLeaf>) -> Self {
        let mut joined: Vec<Leaf> = self.get_leaves().to_vec();
        for leaf in leaves {
            let examples: Vec<Example> = leaf
                .examples
                .into_iter()
                .map(|(words, about)| Example::new(leak_words(words), about.leak()))
                .collect();
            joined.push(Leaf::new(leak_words(leaf.path)).examples(examples.leak()));
        }
        self.leaves(joined.leak())
    }
}

/// Returns `words` as a `'static` slice of `'static` words, leaking them.
fn leak_words(words: Vec<String>) -> &'static [&'static str] {
    let words: Vec<&'static str> = words.into_iter().map(|word| &*word.leak()).collect();
    words.leak()
}
