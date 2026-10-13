//! Tests for the guide's terminal detection setting.

use std::cell::Cell;

use crate::help::Guide;

#[test]
fn a_guide_detects_the_terminal_until_the_tool_turns_detection_off() {
    const ON: Guide = Guide::new();
    const OFF: Guide = Guide::new().without_terminal_detection("Our CI runs in a pseudo-terminal.");

    assert_eq!(ON.get_without_terminal_detection(), None);
    assert!(OFF.get_without_terminal_detection().is_some());
}

#[test]
fn without_terminal_detection_keeps_the_tool_s_reason() {
    const OFF: Guide = Guide::new().without_terminal_detection("Our CI runs in a pseudo-terminal.");

    assert_eq!(
        OFF.get_without_terminal_detection(),
        Some("Our CI runs in a pseudo-terminal.")
    );
}

#[test]
fn with_detection_on_a_guide_reports_what_the_terminal_check_finds() {
    let guide = Guide::new();

    assert!(guide.detects(|| true));
    assert!(!guide.detects(|| false));
}

#[test]
fn with_detection_off_a_guide_never_asks_whether_a_stream_is_a_terminal() {
    let checked = Cell::new(false);

    let detected = Guide::new()
        .without_terminal_detection("Our CI runs in a pseudo-terminal.")
        .detects(|| {
            checked.set(true);
            true
        });

    assert!(!detected);
    assert!(!checked.get());
}
