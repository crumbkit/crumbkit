//! The help size check, which keeps each command's `-h` page within a
//! budget of bytes.

use clap::Command;
use crumbkit::help::Guide;

use crate::budget::{Budget, Departure, Limit};
use crate::pages::pages;
use crate::tree::{self, Visit};

#[cfg(test)]
mod tests;

/// crumbkit's default budget for the `-h` page of the root, in bytes.
const ROOT: u64 = 1_500;

/// crumbkit's default budget for the `-h` page of a group, in bytes.
const GROUP: u64 = 800;

/// crumbkit's default budget for the `-h` page of a leaf, in bytes.
const LEAF: u64 = 1_200;

/// The budgets of a tool's `-h` pages, in bytes, which
/// [`check_help_sizes`] checks.
///
/// [`Budgets::new`] holds crumbkit's default budgets for the root, for a
/// group, which is a command below the root with subcommands, and for a leaf,
/// which is a command below the root without subcommands. [ADR-0025] records
/// the default budgets. The root of a tool without subcommands has the
/// root's budget. A tool can raise or lower the budget of a whole class, or
/// of single commands, and gives a reason for each budget it sets, which
/// [`check_opt_outs`](crate::check_opt_outs) lists:
///
/// ```
/// use crumbkit_test::{Budget, Budgets};
///
/// const BUDGETS: Budgets = Budgets::new()
///     .leaf(1_000, "Leaves describe one action each.")
///     .nodes(&[Budget::new("books list", 1_500, "The listing has many filters.")]);
/// ```
///
/// [ADR-0025]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0025-help-checks.md
#[derive(Clone, Copy, Debug)]
pub struct Budgets {
    /// The budget of the root's page.
    root: Limit,
    /// The budget of each group's page.
    group: Limit,
    /// The budget of each leaf's page.
    leaf: Limit,
    /// The budgets of single commands, which win over the budget of the
    /// command's class.
    nodes: &'static [Budget],
}

impl Budgets {
    /// Returns crumbkit's default budgets for the root, each group and each
    /// leaf, which [ADR-0025](https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/0025-help-checks.md) records, and no budget for a single
    /// command.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            root: Limit::default_of(ROOT),
            group: Limit::default_of(GROUP),
            leaf: Limit::default_of(LEAF),
            nodes: &[],
        }
    }

    /// Sets the budget of the root's page to `bytes`, for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, so a
    /// `const` that passes such a reason fails to compile.
    #[must_use]
    pub const fn root(mut self, bytes: u64, reason: &'static str) -> Self {
        self.root = Limit::set(bytes, reason);
        self
    }

    /// Sets the budget of each group's page to `bytes`, for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, so a
    /// `const` that passes such a reason fails to compile.
    #[must_use]
    pub const fn group(mut self, bytes: u64, reason: &'static str) -> Self {
        self.group = Limit::set(bytes, reason);
        self
    }

    /// Sets the budget of each leaf's page to `bytes`, for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, so a
    /// `const` that passes such a reason fails to compile.
    #[must_use]
    pub const fn leaf(mut self, bytes: u64, reason: &'static str) -> Self {
        self.leaf = Limit::set(bytes, reason);
        self
    }

    /// Sets the budgets of single commands. A command's own budget wins
    /// over the budget of its class, and the first entry for a path wins
    /// over a later one.
    #[must_use]
    pub const fn nodes(mut self, nodes: &'static [Budget]) -> Self {
        self.nodes = nodes;
        self
    }

    /// Returns each budget that the tool set, for the tool `command`, and
    /// adds a problem to `problems` for a command's budget whose path names
    /// no visible command below the root.
    pub(crate) fn departures(
        &self,
        command: &Command,
        problems: &mut Vec<String>,
    ) -> Vec<Departure> {
        let name = command.get_name();
        let mut departures = Vec::new();
        for class in [Class::Root, Class::Group, Class::Leaf] {
            let limit = class.limit(self);
            if let Some(reason) = limit.reason {
                departures.push(Departure {
                    rule: class.rule(),
                    applies_to: class.applies_to(name),
                    reason,
                    bytes: limit.bytes,
                    default: class.default_budget(),
                });
            }
        }
        let visible = tree::walk_visible(command);
        for budget in self.nodes {
            match find(&visible, budget.path()) {
                Some(visit) => departures.push(Departure {
                    rule: "help-budget/command",
                    applies_to: visit.path().to_owned(),
                    reason: budget.reason(),
                    bytes: budget.bytes(),
                    default: Class::of(visit).default_budget(),
                }),
                None => problems.push(names_nothing(budget)),
            }
        }
        departures
    }
}

impl Default for Budgets {
    /// Returns crumbkit's default budgets, as [`Budgets::new`] does.
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the visit in `visible` of the command at `path` below the root.
pub(crate) fn find<'v, 'c>(visible: &'v [Visit<'c>], path: &str) -> Option<&'v Visit<'c>> {
    visible
        .iter()
        .find(|visit| visit.depth() > 0 && visit.below() == path)
}

/// Returns the problem of `budget`, whose path names no visible command
/// below the root.
pub(crate) fn names_nothing(budget: &Budget) -> String {
    format!(
        "the budget of `{}` names no visible command below the root",
        budget.path()
    )
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

    /// Returns the id of the class's budget in the opt-out report.
    fn rule(self) -> &'static str {
        match self {
            Self::Root => "help-budget/root",
            Self::Group => "help-budget/group",
            Self::Leaf => "help-budget/leaf",
        }
    }

    /// Returns what the class's budget applies to in the tool `tool`, as
    /// the opt-out report names it.
    fn applies_to(self, tool: &str) -> String {
        match self {
            Self::Root => tool.to_owned(),
            Self::Group => format!("every group of {tool}"),
            Self::Leaf => format!("every leaf of {tool}"),
        }
    }

    /// Returns crumbkit's default budget for the class, in bytes.
    fn default_budget(self) -> u64 {
        match self {
            Self::Root => ROOT,
            Self::Group => GROUP,
            Self::Leaf => LEAF,
        }
    }

    /// Returns the class's budget in `budgets`.
    fn limit(self, budgets: &Budgets) -> Limit {
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
    let pages = pages(command, guide, "-h", &mut problems);
    // A command whose page failed to render has no entry in `pages`, so the
    // budgets are matched against the visible tree, and that command's one
    // problem is the page that `pages` reports.
    let visible = tree::walk_visible(command);
    for budget in budgets.nodes {
        if find(&visible, budget.path()).is_none() {
            problems.push(names_nothing(budget));
        }
    }
    for (visit, page) in &pages {
        let own = budgets
            .nodes
            .iter()
            .find(|budget| visit.depth() > 0 && budget.path() == visit.below());
        let budget = match own {
            Some(budget) => budget.bytes(),
            None => Class::of(visit).limit(budgets).bytes,
        };
        if page.len() as u64 > budget {
            problems.push(format!(
                "{} -h: the page has {} bytes, over its budget of {budget} bytes",
                visit.path(),
                page.len()
            ));
        }
    }
    tree::report("help size rules", &problems);
}
