//! Runtime leaves, which give examples to the commands that a tool adds to
//! its command tree at run time, such as one command for each file that a
//! person writes.

use super::{Effects, Example, Guide, Leaf, RefusalContext};
use crate::{Call, Document, Rows};

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
/// A command added at run time prints a document that the tool declares as
/// a constant, such as the document of a static command that does the same
/// work. [`RuntimeLeaf::document`] declares it, as [`Leaf::document`] does,
/// so the command's `--help` lists the columns, `commands list` gives it
/// the document's columns and form, and `schemas list` names it among the
/// commands that print the document (ADR-0080). A runtime leaf without a
/// document lists no columns.
#[derive(Clone, Debug, Default)]
pub struct RuntimeLeaf {
    /// The names of the subcommands from the root to the command.
    path: Vec<String>,
    /// The examples, each the words after the path and a description, in
    /// the order the help page lists them.
    examples: Vec<(Vec<String>, String)>,
    /// A leaf without a path that holds the document the command prints,
    /// as [`Leaf::document`] set it, or `None` when the leaf has no
    /// document.
    document: Option<Leaf>,
    /// The command's effects, or `None` when the tool declares none.
    effects: Option<Effects>,
    /// The program whose exit code the command passes on, or `None`.
    passes_on_exit: Option<&'static str>,
    /// The function that builds the calls that a refusal of the command's
    /// line offers as its recovery and alternatives, or `None`.
    refusal_alternatives: Option<fn(&RefusalContext<'_>) -> Vec<Call>>,
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
            document: None,
            effects: None,
            passes_on_exit: None,
            refusal_alternatives: None,
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

    /// Sets the document that the command prints, the same constant that
    /// the tool passes to [`finish`](crate::finish), as [`Leaf::document`]
    /// does. The command's `--help` then lists the columns of the
    /// document's row type, and [`schemas`](crate::schemas) serves the
    /// document with the command among those that print it.
    ///
    /// The document's name usually follows the path of a static command,
    /// so it differs from the path of the command added at run time.
    /// crumbkit-test's `check_opt_outs` then lists the name among the
    /// tool's customisations, as it lists a `const` leaf's.
    #[must_use]
    pub const fn document<T: Rows>(mut self, document: Document<T>) -> Self {
        self.document = Some(Leaf::new(&[]).document(document));
        self
    }

    /// Sets the command's effects, as [`Leaf::effects`] does.
    #[must_use]
    pub const fn effects(mut self, effects: Effects) -> Self {
        self.effects = Some(effects);
        self
    }

    /// Declares that the command passes on the exit code of `program`, as
    /// [`Leaf::passes_on_exit`] does.
    #[must_use]
    pub const fn passes_on_exit(mut self, program: &'static str) -> Self {
        self.passes_on_exit = Some(program);
        self
    }

    /// Sets the function that builds the calls that a refusal of a command
    /// line naming the command offers as its recovery and alternatives, as
    /// [`Leaf::refusal_alternatives`] does.
    #[must_use]
    pub const fn refusal_alternatives(
        mut self,
        alternatives: fn(&RefusalContext<'_>) -> Vec<Call>,
    ) -> Self {
        self.refusal_alternatives = Some(alternatives);
        self
    }
}

impl Guide {
    /// Returns a guide with every setting of `self`, and the leaves of
    /// `self` followed by `leaves`, each with its path, examples, document,
    /// effects, the program whose exit code it passes on and its refusal
    /// alternatives. The joined guide keeps the renames that
    /// [`Guide::codes`] set, so a failure or refusal of a runtime leaf's
    /// command prints the tool's code in place of crumbkit's, as one of a
    /// `const` leaf's command does. It also keeps the groups, topics, exit
    /// codes, failure form and every other setting of `self`.
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
            let mut joined_leaf = Leaf::new(leak_words(leaf.path)).examples(examples.leak());
            if let Some(documented) = leaf.document {
                joined_leaf = joined_leaf.with_document_of(&documented);
            }
            if let Some(effects) = leaf.effects {
                joined_leaf = joined_leaf.effects(effects);
            }
            if let Some(program) = leaf.passes_on_exit {
                joined_leaf = joined_leaf.passes_on_exit(program);
            }
            if let Some(alternatives) = leaf.refusal_alternatives {
                joined_leaf = joined_leaf.refusal_alternatives(alternatives);
            }
            joined.push(joined_leaf);
        }
        self.leaves(joined.leak())
    }
}

/// Returns `words` as a `'static` slice of `'static` words, leaking them.
fn leak_words(words: Vec<String>) -> &'static [&'static str] {
    let words: Vec<&'static str> = words.into_iter().map(|word| &*word.leak()).collect();
    words.leak()
}
