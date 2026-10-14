//! The budget of one command, which a tool sets with a stated reason, and
//! the budgets that differ from crumbkit's defaults, which the opt-out
//! report lists.

use crate::allow::is_reason;

#[cfg(test)]
mod tests;

/// The budget in bytes of one command, named by its path below the root,
/// with the tool's reason for setting it.
///
/// [`Budgets::nodes`](crate::Budgets::nodes) takes the budgets of single
/// `-h` pages, and [`PathBudgets::nodes`](crate::PathBudgets::nodes) the
/// budgets of single paths. A budget can be above or below crumbkit's
/// default, and [`check_opt_outs`](crate::check_opt_outs) lists it with its
/// reason:
///
/// ```
/// use crumbkit_test::{Budget, Budgets};
///
/// const BUDGETS: Budgets =
///     Budgets::new().nodes(&[Budget::new("books list", 1_500, "The listing has many filters.")]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    /// The command's path below the root, with its names joined by single
    /// spaces, such as `books list`.
    path: &'static str,
    /// The budget, in bytes.
    bytes: u64,
    /// Why the tool sets the budget, in one line.
    reason: &'static str,
}

impl Budget {
    /// Returns the budget of `bytes` for the command at `path`, its path
    /// below the root with its names joined by single spaces, such as
    /// `"books list"`, for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A
    /// `const` that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit_test::Budget;
    ///
    /// const BAD: Budget = Budget::new("books list", 1_500, "");
    /// ```
    #[must_use]
    pub const fn new(path: &'static str, bytes: u64, reason: &'static str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            path,
            bytes,
            reason,
        }
    }

    /// Returns the command's path below the root.
    pub(crate) const fn path(&self) -> &'static str {
        self.path
    }

    /// Returns the budget, in bytes.
    pub(crate) const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Returns the tool's reason.
    pub(crate) const fn reason(&self) -> &'static str {
        self.reason
    }
}

/// The budget of a class of pages or paths, in bytes, and the tool's reason
/// when it set the budget.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Limit {
    /// The budget, in bytes.
    pub(crate) bytes: u64,
    /// The tool's reason, or `None` for crumbkit's default.
    pub(crate) reason: Option<&'static str>,
}

impl Limit {
    /// Returns crumbkit's default budget of `bytes`.
    pub(crate) const fn default_of(bytes: u64) -> Self {
        Self {
            bytes,
            reason: None,
        }
    }

    /// Returns the budget of `bytes` that a tool set for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break.
    pub(crate) const fn set(bytes: u64, reason: &'static str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            bytes,
            reason: Some(reason),
        }
    }
}

/// A budget that a tool set, which the opt-out report lists.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Departure {
    /// The rule's id, such as `help-budget/leaf`.
    pub(crate) rule: &'static str,
    /// What the budget applies to, such as `every leaf of shelf` or
    /// `shelf books list`.
    pub(crate) applies_to: String,
    /// The tool's reason.
    pub(crate) reason: &'static str,
    /// The budget, in bytes.
    pub(crate) bytes: u64,
    /// crumbkit's default for the same pages or paths, in bytes.
    pub(crate) default: u64,
}

impl Departure {
    /// Returns the bytes by which the budget is above crumbkit's default, or
    /// 0 for a budget at or below the default.
    pub(crate) const fn extra_bytes(&self) -> u64 {
        self.bytes.saturating_sub(self.default)
    }

    /// Returns [`Departure::extra_bytes`] in tokens at 4 bytes a token,
    /// rounded up, as `docs/behaviour/help/checks.md` counts tokens.
    pub(crate) const fn extra_tokens(&self) -> u64 {
        self.extra_bytes().div_ceil(4)
    }
}
