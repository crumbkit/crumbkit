//! Tests for the terminal detection setting of the output record.

use std::cell::Cell;

use crate::Tool;
use crate::tool::Output;

#[test]
fn a_tool_detects_the_terminal_until_the_tool_turns_detection_off() {
    const ON: Tool = Tool::new();
    const OFF: Tool = Tool::new()
        .output(Output::new().without_terminal_detection("Our CI runs in a pseudo-terminal."));

    assert_eq!(ON.get_output().get_without_terminal_detection(), None);
    assert!(OFF.get_output().get_without_terminal_detection().is_some());
}

#[test]
fn without_terminal_detection_keeps_the_tool_s_reason() {
    const OFF: Tool = Tool::new()
        .output(Output::new().without_terminal_detection("Our CI runs in a pseudo-terminal."));

    assert_eq!(
        OFF.get_output().get_without_terminal_detection(),
        Some("Our CI runs in a pseudo-terminal.")
    );
}

#[test]
fn with_detection_on_a_tool_reports_what_the_terminal_check_finds() {
    let tool = Tool::new();

    assert!(tool.get_output().detects(|| true));
    assert!(!tool.get_output().detects(|| false));
}

#[test]
fn with_detection_off_a_tool_never_asks_whether_a_stream_is_a_terminal() {
    let checked = Cell::new(false);

    let detected = Output::new()
        .without_terminal_detection("Our CI runs in a pseudo-terminal.")
        .detects(|| {
            checked.set(true);
            true
        });

    assert!(!detected);
    assert!(!checked.get());
}
