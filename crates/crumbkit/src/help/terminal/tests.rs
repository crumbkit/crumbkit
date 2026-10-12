//! Tests for the guide's terminal detection setting.

use std::cell::Cell;

use crate::help::Guide;

#[test]
fn a_guide_detects_the_terminal_until_the_tool_turns_detection_off() {
    const ON: Guide = Guide::new();
    const OFF: Guide = Guide::new().without_terminal_detection();

    assert!(ON.get_terminal_detection());
    assert!(!OFF.get_terminal_detection());
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

    let detected = Guide::new().without_terminal_detection().detects(|| {
        checked.set(true);
        true
    });

    assert!(!detected);
    assert!(!checked.get());
}
