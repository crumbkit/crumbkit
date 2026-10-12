//! Tests for `Output::progress_allowed`.

use std::cell::Cell;

use super::allowed;

#[test]
fn progress_is_allowed_at_a_terminal_without_json() {
    assert!(allowed(false, true, || true));
}

#[test]
fn progress_is_not_allowed_with_json_or_without_a_terminal() {
    assert!(!allowed(true, true, || true));
    assert!(!allowed(false, true, || false));
}

#[test]
fn progress_is_never_allowed_with_terminal_detection_off_even_at_a_terminal() {
    let checked = Cell::new(false);

    let allowed = allowed(false, false, || {
        checked.set(true);
        true
    });

    assert!(!allowed);
    assert!(!checked.get());
}
