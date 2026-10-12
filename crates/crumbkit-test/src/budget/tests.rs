//! Tests for budgets with reasons.

use super::{Budget, Departure};

/// Returns the departure of a budget of `bytes` from a default of
/// `default`.
fn departure(bytes: u64, default: u64) -> Departure {
    Departure {
        rule: "help-budget/leaf",
        applies_to: "every leaf of shelf".to_owned(),
        reason: "A test.",
        bytes,
        default,
    }
}

#[test]
fn a_budget_keeps_its_path_bytes_and_reason() {
    const BUDGET: Budget = Budget::new("books list", 1_500, "The listing has many filters.");

    assert_eq!(
        (BUDGET.path(), BUDGET.bytes(), BUDGET.reason()),
        ("books list", 1_500, "The listing has many filters.")
    );
}

#[test]
fn a_raised_budget_counts_its_extra_bytes_and_tokens_rounded_up() {
    assert_eq!(departure(1_201, 1_200).extra_bytes(), 1);
    assert_eq!(departure(1_201, 1_200).extra_tokens(), 1);
    assert_eq!(departure(1_208, 1_200).extra_tokens(), 2);
}

#[test]
fn a_lowered_budget_has_no_extra_bytes() {
    assert_eq!(departure(600, 800).extra_bytes(), 0);
    assert_eq!(departure(600, 800).extra_tokens(), 0);
}
