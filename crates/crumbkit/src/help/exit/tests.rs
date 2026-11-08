//! Tests for declared exit codes: which commands a code covers, which
//! codes a range holds, the text of `help exit-codes` and the note of
//! `commands list`.

use super::{Exit, note, text};
use crate::help::{Guide, Leaf};

/// A code that every command returns.
const BUSY: Exit = Exit::new(3, "The catalogue was locked by another run.");

/// A code that two branches return.
const TIMED_OUT: Exit =
    Exit::new(124, "The download ran out of time.").commands(&[&["books", "import"], &["sync"]]);

/// A range of codes that the downloader passes on.
const PASSED_ON: Exit = Exit::range(124, 125, "The downloader ended with its own code.");

#[test]
fn a_range_holds_each_code_from_its_first_to_its_last_and_nothing_outside() {
    assert_eq!(PASSED_ON.codes(), 124..=125);
    assert!(PASSED_ON.codes().contains(&124));
    assert!(PASSED_ON.codes().contains(&125));
    assert!(!PASSED_ON.codes().contains(&123));
    assert!(!PASSED_ON.codes().contains(&126));
    assert_eq!(PASSED_ON.code(), 124);
}

#[test]
fn a_single_code_holds_only_itself() {
    assert_eq!(BUSY.codes(), 3..=3);
    assert_eq!(BUSY.code(), 3);
}

#[test]
fn the_topic_prints_a_range_as_its_first_and_last_code_joined_by_a_dash_beside_a_single_code() {
    const GUIDE: Guide =
        Guide::new().exits(&[BUSY, PASSED_ON, Exit::new(130, "Ctrl-C ended the run.")]);

    let expected = "\
Exit codes
  0        The command did what it was asked.
  1        The command failed, or a row of its result reports a problem. A failure names its code and the next step.
  2        The command line was refused. The refusal names the help to read.
  3        The catalogue was locked by another run.
  124-125  The downloader ended with its own code.
  130      Ctrl-C ended the run.

A shell reports 126 and above for its own reasons, such as 127 when it cannot find shelf and 130 when Ctrl-C ends it.
";
    assert_eq!(text("shelf", &GUIDE), expected);
}

#[test]
fn the_note_names_a_range_by_its_first_and_last_code() {
    const GUIDE: Guide = Guide::new().exits(&[BUSY, PASSED_ON]);

    assert_eq!(
        note("shelf", &GUIDE),
        "Exit codes: 0, 1 and 2 apply to every command, and shelf adds 3 and 124-125. `shelf help exit-codes` describes them."
    );
}

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
  1  The command failed, or a row of its result reports a problem. A failure names its code and the next step.
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
  1    The command failed, or a row of its result reports a problem. A failure names its code and the next step.
  2    The command line was refused. The refusal names the help to read.
  3    The catalogue was locked by another run.
  124  The download ran out of time. Commands: `shelf books import`, `shelf sync`.

A shell reports 126 and above for its own reasons, such as 127 when it cannot find shelf and 130 when Ctrl-C ends it.
";
    assert_eq!(text("shelf", &GUIDE), expected);
}

#[test]
fn the_note_names_the_codes_of_every_command_and_the_topic_when_the_tool_declares_none() {
    assert_eq!(
        note("shelf", &Guide::new()),
        "Exit codes: 0, 1 and 2 apply to every command. `shelf help exit-codes` describes them."
    );
}

#[test]
fn the_note_names_each_of_the_tool_s_codes() {
    const ONE: Guide = Guide::new().exits(&[TIMED_OUT]);
    const THREE: Guide = Guide::new().exits(&[BUSY, Exit::new(4, "Four."), TIMED_OUT]);

    assert_eq!(
        note("shelf", &ONE),
        "Exit codes: 0, 1 and 2 apply to every command, and shelf adds 124. `shelf help exit-codes` describes them."
    );
    assert_eq!(
        note("shelf", &THREE),
        "Exit codes: 0, 1 and 2 apply to every command, and shelf adds 3, 4 and 124. `shelf help exit-codes` describes them."
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
  1   The command failed, or a row of its result reports a problem. A failure names its code and the next step.
  64  The command line was refused. The refusal names the help to read.

A shell reports 126 and above for its own reasons, such as 127 when it cannot find shelf and 130 when Ctrl-C ends it.
";
    assert_eq!(text("shelf", &USAGE_64), expected);
}

#[test]
fn the_note_names_0_and_1_as_applying_to_every_command_when_the_guide_gives_a_refusal_another_code()
{
    assert_eq!(
        note("shelf", &USAGE_64),
        "Exit codes: 0 and 1 apply to every command, and shelf adds 64. `shelf help exit-codes` describes them."
    );
}

#[test]
fn the_topic_names_each_command_that_passes_on_a_program_s_exit_code() {
    const GUIDE: Guide = Guide::new().exits(&[BUSY]).leaves(&[
        Leaf::new(&["books", "edit"]).passes_on_exit("the editor"),
        Leaf::new(&["books", "list"]),
    ]);

    let expected = "\
Exit codes
  0  The command did what it was asked.
  1  The command failed, or a row of its result reports a problem. A failure names its code and the next step.
  2  The command line was refused. The refusal names the help to read.
  3  The catalogue was locked by another run.

`shelf books edit` passes on the exit code of the editor.

A shell reports 126 and above for its own reasons, such as 127 when it cannot find shelf and 130 when Ctrl-C ends it.
";
    assert_eq!(text("shelf", &GUIDE), expected);
}
