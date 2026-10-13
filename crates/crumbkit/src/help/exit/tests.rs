//! Tests for declared exit codes: which commands a code covers, the text
//! of `help exit-codes` and the note of `commands list`.

use super::{Exit, note, text};
use crate::help::Guide;

/// A code that every command returns.
const BUSY: Exit = Exit::new(3, "The catalogue was locked by another run.");

/// A code that two branches return.
const TIMED_OUT: Exit =
    Exit::new(124, "The download ran out of time.").commands(&[&["books", "import"], &["sync"]]);

#[test]
fn a_code_without_commands_covers_every_command_and_the_root() {
    assert!(BUSY.covers(&[]));
    assert!(BUSY.covers(&["books", "list"]));
}

#[test]
fn a_code_with_commands_covers_each_named_command_and_the_commands_below_it() {
    assert!(TIMED_OUT.covers(&["books", "import"]));
    assert!(TIMED_OUT.covers(&["sync", "now"]));
    assert!(!TIMED_OUT.covers(&["books"]));
    assert!(!TIMED_OUT.covers(&["books", "list"]));
    assert!(!TIMED_OUT.covers(&[]));
}

#[test]
fn the_topic_lists_crumbkit_s_codes_when_the_tool_declares_none() {
    let expected = "\
Exit codes
  0  The command did what it was asked.
  1  The command failed. The failure names its code and the next step.
  2  The command line was refused. The refusal names the help to read.

A shell reports 126 and above for its own reasons, such as 127 when it cannot find shelf and 130 when Ctrl-C ends it.
";
    assert_eq!(text("shelf", &Guide::new()), expected);
}

#[test]
fn the_topic_lists_the_tool_s_codes_after_crumbkit_s_with_the_commands_that_return_them() {
    const GUIDE: Guide = Guide::new().exits(&[BUSY, TIMED_OUT]);

    let expected = "\
Exit codes
  0    The command did what it was asked.
  1    The command failed. The failure names its code and the next step.
  2    The command line was refused. The refusal names the help to read.
  3    The catalogue was locked by another run.
  124  The download ran out of time. Commands: `shelf books import`, `shelf sync`.

A shell reports 126 and above for its own reasons, such as 127 when it cannot find shelf and 130 when Ctrl-C ends it.
";
    assert_eq!(text("shelf", &GUIDE), expected);
}

#[test]
fn the_note_names_crumbkit_s_codes_and_the_topic_when_the_tool_declares_none() {
    assert_eq!(
        note("shelf", &Guide::new()),
        "Exit codes: 0, 1 and 2 are crumbkit's. `shelf help exit-codes` describes them."
    );
}

#[test]
fn the_note_names_each_of_the_tool_s_codes() {
    const ONE: Guide = Guide::new().exits(&[TIMED_OUT]);
    const THREE: Guide = Guide::new().exits(&[BUSY, Exit::new(4, "Four."), TIMED_OUT]);

    assert_eq!(
        note("shelf", &ONE),
        "Exit codes: 0, 1 and 2 are crumbkit's, and shelf adds 124. `shelf help exit-codes` describes them."
    );
    assert_eq!(
        note("shelf", &THREE),
        "Exit codes: 0, 1 and 2 are crumbkit's, and shelf adds 3, 4 and 124. `shelf help exit-codes` describes them."
    );
}

/// A guide that gives a refused command line 64 in place of 2.
const USAGE_64: Guide = Guide::new().exits(&[Exit::new(
    64,
    "The command line was refused. The refusal names the help to read.",
)
.failures(&[&crate::codes::CLI_USAGE])]);

#[test]
fn the_topic_leaves_out_crumbkit_s_2_when_the_guide_gives_a_refusal_another_code() {
    let expected = "\
Exit codes
  0   The command did what it was asked.
  1   The command failed. The failure names its code and the next step.
  64  The command line was refused. The refusal names the help to read.

A shell reports 126 and above for its own reasons, such as 127 when it cannot find shelf and 130 when Ctrl-C ends it.
";
    assert_eq!(text("shelf", &USAGE_64), expected);
}

#[test]
fn the_note_names_0_and_1_as_crumbkit_s_when_the_guide_gives_a_refusal_another_code() {
    assert_eq!(
        note("shelf", &USAGE_64),
        "Exit codes: 0 and 1 are crumbkit's, and shelf adds 64. `shelf help exit-codes` describes them."
    );
}
