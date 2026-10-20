//! Tests for `Output::progress_allowed` and the terminal width that a text
//! table fits.

use std::cell::Cell;

use super::{allowed, width};

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

#[test]
fn the_terminal_width_is_read_with_terminal_detection_on() {
    assert_eq!(width(true, || Some(80)), Some(80));
    assert_eq!(width(true, || None), None);
}

#[test]
fn the_terminal_width_is_never_read_with_terminal_detection_off() {
    let read = Cell::new(false);

    let width = width(false, || {
        read.set(true);
        Some(80)
    });

    assert_eq!(width, None);
    assert!(!read.get(), "the width was read with detection off");
}
