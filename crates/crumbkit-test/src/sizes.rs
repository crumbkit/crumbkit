//! The help size check, which keeps each command's `-h` page within a
//! budget of bytes.

use clap::Command;
use crumbkit::help::Guide;

use crate::pages::pages;
use crate::tree::{self, Visit};

#[cfg(test)]
mod tests;

/// crumbkit's default budget for the `-h` page of the root, in bytes.
const ROOT: usize = 1_500;

/// crumbkit's default budget for the `-h` page of a group, in bytes.
const GROUP: usize = 800;

/// crumbkit's default budget for the `-h` page of a leaf, in bytes.
const LEAF: usize = 1_200;

/// The budgets of a tool's `-h` pages, in bytes, which
/// [`check_help_sizes`] checks.
///
/// [`Budgets::new`] holds crumbkit's default budgets for the root, for a
/// group, which is a command below the root with subcommands, and for a leaf,
/// which is a command below the root without subcommands. [ADR-0025] records
/// the default budgets. The root of a tool without subcommands has the
/// root's budget. A tool can lower a whole class, or single commands, and
/// cannot raise a budget above crumbkit's default:
///
/// ```
/// use crumbkit_test::Budgets;
///
/// const BUDGETS: Budgets = Budgets::new().leaf(1_000).nodes(&[("books list", 600)]);
/// ```
///
/// [ADR-0025]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0025-help-checks.md
#[derive(Clone, Copy, Debug)]
pub struct Budgets {
    /// The budget of the root's page.
    root: usize,
    /// The budget of each group's page.
    group: usize,
    /// The budget of each leaf's page.
    leaf: usize,
    /// The budgets of single commands, each named by its path below the
    /// root, which win over the budget of the command's class.
    nodes: &'static [(&'static str, usize)],
}

impl Budgets {
    /// Returns crumbkit's default budgets for the root, each group and each
    /// leaf, which [ADR-0025](https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0025-help-checks.md) records, and no budget for a single
    /// command.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: ROOT,
            group: GROUP,
            leaf: LEAF,
            nodes: &[],
        }
    }

    /// Sets the budget of the root's page to `bytes`.
    #[must_use]
    pub const fn root(mut self, bytes: usize) -> Self {
        self.root = bytes;
        self
    }

    /// Sets the budget of each group's page to `bytes`.
    #[must_use]
    pub const fn group(mut self, bytes: usize) -> Self {
        self.group = bytes;
        self
    }

    /// Sets the budget of each leaf's page to `bytes`.
    #[must_use]
    pub const fn leaf(mut self, bytes: usize) -> Self {
        self.leaf = bytes;
        self
    }

    /// Sets the budgets of single commands, each a command's path below the
    /// root, with its names joined by single spaces, such as
    /// `"books list"`, and a number of bytes. A command's own budget wins
    /// over the budget of its class, and the first entry for a path wins
    /// over a later one.
    #[must_use]
    pub const fn nodes(mut self, nodes: &'static [(&'static str, usize)]) -> Self {
        self.nodes = nodes;
        self
    }
}

impl Default for Budgets {
    /// Returns crumbkit's default budgets, as [`Budgets::new`] does.
    fn default() -> Self {
        Self::new()
    }
}

/// The class of a command, which sets its default budget.
#[derive(Clone, Copy)]
enum Class {
    /// The command passed to the check.
    Root,
    /// A command below the root with subcommands.
    Group,
    /// A command below the root without subcommands.
    Leaf,
}

impl Class {
    /// Returns the class of the command that `visit` reached.
    fn of(visit: &Visit<'_>) -> Self {
        if visit.depth() == 0 {
            Self::Root
        } else if visit.command().has_subcommands() {
            Self::Group
        } else {
            Self::Leaf
        }
    }

    /// Returns the class's name, as a problem names it.
    fn name(self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Group => "group",
            Self::Leaf => "leaf",
        }
    }

    /// Returns crumbkit's default budget for the class, in bytes.
    fn default_budget(self) -> usize {
        match self {
            Self::Root => ROOT,
            Self::Group => GROUP,
            Self::Leaf => LEAF,
        }
    }

    /// Returns the class's budget in `budgets`, in bytes.
    fn budget(self, budgets: &Budgets) -> usize {
        match self {
            Self::Root => budgets.root,
            Self::Group => budgets.group,
            Self::Leaf => budgets.leaf,
        }
    }
}

/// Checks that the `-h` page of `command` and of each visible command below
/// it fits its budget in `budgets`, and panics, as a failed test assertion
/// does, naming every problem it finds.
///
/// A page is the plain text that `crumbkit::parse` prints for the command's
/// path and `-h` with `guide`, as a caller reads it on a pipe, and its size
/// is its number of bytes. `--help` has no budget, because it describes
/// every flag and column in full. The check skips each hidden command and
/// everything below it, because help never shows them. It finds:
///
/// - a page larger than its budget, naming the command, the page's size and
///   the budget;
/// - a class budget or a command's budget above crumbkit's default for its
///   class, because a tool can lower a budget and cannot raise one;
/// - a command's budget whose path names no command or a hidden one;
/// - a visible command whose `-h` prints no help page, such as one that
///   turned its help flag off.
///
/// ```
/// use clap::Command;
/// use crumbkit::help::Guide;
/// use crumbkit_test::Budgets;
///
/// let shelf = Command::new("shelf")
///     .about("Browse a small catalogue")
///     .subcommand(Command::new("books").subcommand(Command::new("list")));
///
/// crumbkit_test::check_help_sizes(&shelf, &Guide::new(), &Budgets::new());
/// ```
///
/// # Panics
///
/// Panics when a page or a budget breaks a rule above. The message lists
/// each problem on its own line and names each command by its path from
/// `command`, such as `shelf books list`.
pub fn check_help_sizes(command: &Command, guide: &Guide, budgets: &Budgets) {
    let mut problems = Vec::new();
    for class in [Class::Root, Class::Group, Class::Leaf] {
        let budget = class.budget(budgets);
        let default = class.default_budget();
        if budget > default {
            problems.push(format!(
                "the {} budget of {budget} bytes is above crumbkit's default of {default} bytes, and a tool can lower a budget and cannot raise one",
                class.name()
            ));
        }
    }
    let pages = pages(command, guide, "-h", &mut problems);
    // A command whose page failed to render has no entry in `pages`, so the
    // budgets are matched against the visible tree, and that command's one
    // problem is the page that `pages` reports.
    let visible = tree::walk_visible(command);
    for &(path, budget) in budgets.nodes {
        let visit = visible
            .iter()
            .find(|visit| visit.depth() > 0 && visit.below() == path);
        let Some(visit) = visit else {
            problems.push(format!(
                "the budget of `{path}` names no visible command below the root"
            ));
            continue;
        };
        let class = Class::of(visit);
        let default = class.default_budget();
        if budget > default {
            problems.push(format!(
                "{}: the budget of {budget} bytes is above crumbkit's default of {default} bytes for a {}, and a tool can lower a budget and cannot raise one",
                visit.path(),
                class.name()
            ));
        }
    }
    for (visit, page) in &pages {
        let own = budgets
            .nodes
            .iter()
            .find(|(path, _)| visit.depth() > 0 && *path == visit.below());
        let budget = match own {
            Some(&(_, bytes)) => bytes,
            None => Class::of(visit).budget(budgets),
        };
        if page.len() > budget {
            problems.push(format!(
                "{} -h: the page has {} bytes, over its budget of {budget} bytes",
                visit.path(),
                page.len()
            ));
        }
    }
    tree::report("help size rules", &problems);
}
