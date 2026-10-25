use std::ffi::OsStr;
use std::io;

use clap::ColorChoice;

use super::role::{Deliver as _, Message, Role};
use super::{ColorVariables, Sink, clicolor_off, color_forced, color_off, term_dumb, wants_styles};

/// Every kind of message, so a test covers each one.
const KINDS: [Message; 9] = [
    Message::Refusal,
    Message::RefusalHelp,
    Message::Failure,
    Message::Note,
    Message::DebugWarning,
    Message::DryRun,
    Message::Prompt,
    Message::Echo,
    Message::FailedWrite,
];

/// Fails to compile when a kind of message is added, so [`KINDS`] lists
/// every kind.
const fn listed(kind: Message) {
    match kind {
        Message::Refusal
        | Message::RefusalHelp
        | Message::Failure
        | Message::Note
        | Message::DebugWarning
        | Message::DryRun
        | Message::Prompt
        | Message::Echo
        | Message::FailedWrite => {}
    }
}

#[test]
fn a_result_delivers_to_the_output_writer() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut sink = Sink::new(&mut out, &mut err);

    sink.deliver(Role::Result, b"rows\n").unwrap();

    assert_eq!((out, err), (b"rows\n".to_vec(), Vec::new()));
}

#[test]
fn every_message_kind_delivers_to_the_error_writer() {
    for kind in KINDS {
        listed(kind);
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut sink = Sink::new(&mut out, &mut err);

        sink.deliver(Role::Message(kind), b"note\n").unwrap();

        assert_eq!((out, err), (Vec::new(), b"note\n".to_vec()), "{kind:?}");
    }
}

#[test]
fn a_results_bytes_are_counted_and_a_messages_are_not() {
    let mut sink = Sink::new(Vec::new(), Vec::new());

    sink.deliver(Role::Result, b"rows\n").unwrap();
    sink.deliver(Role::Message(Message::Note), b"note: one\n")
        .unwrap();
    sink.deliver(Role::Result, b"more\n").unwrap();

    assert_eq!(sink.result_bytes(), 10);
}

#[test]
fn a_result_that_a_writer_takes_in_part_counts_the_bytes_it_took() {
    let mut sink = Sink::new(Short { room: 3 }, Vec::new());

    let error = sink.deliver(Role::Result, b"rows\n").unwrap_err();

    assert_eq!(
        (error.kind(), sink.result_bytes()),
        (io::ErrorKind::WriteZero, 3)
    );
}

#[test]
fn only_the_result_wants_styles_on_a_styled_sink() {
    let styled = Sink::new(Vec::new(), Vec::new()).styled(true);
    let plain = Sink::new(Vec::new(), Vec::new());

    assert!(styled.styles(Role::Result));
    assert!(!plain.styles(Role::Result));
    for kind in KINDS {
        assert!(!styled.styles(Role::Message(kind)), "{kind:?}");
    }
}

#[test]
fn a_sink_whose_error_writer_wants_styles_styles_a_refusal_and_its_group_page_and_no_other_message()
{
    let sink = Sink::new(Vec::new(), Vec::new()).styled_errors(true);

    assert!(!sink.styles(Role::Result));
    for kind in KINDS {
        let refusal = matches!(kind, Message::Refusal | Message::RefusalHelp);
        assert_eq!(sink.styles(Role::Message(kind)), refusal, "{kind:?}");
    }
}

#[test]
fn a_sink_asks_for_no_wants_styles_until_its_builder_asks_for_them() {
    let sink = Sink::new(Vec::new(), Vec::new());
    assert!(!sink.is_styled_errors());

    let sink = sink.styled_errors(true);
    assert!(sink.is_styled_errors());
    assert!(!sink.is_styled(), "error styles leave the result plain");

    let sink = sink.styled_errors(false);
    assert!(!sink.is_styled_errors());
}

#[test]
fn a_sink_whose_result_is_styled_asks_for_no_wants_styles() {
    let sink = Sink::new(Vec::new(), Vec::new()).styled(true);

    assert!(!sink.is_styled_errors());
    assert!(!sink.styles(Role::Message(Message::Refusal)));
}

#[test]
fn a_sink_from_new_or_with_input_styles_no_message() {
    let new = Sink::new(Vec::new(), Vec::new()).styled(true);
    let with_input = Sink::with_input(Vec::new(), Vec::new(), &b""[..], true);

    for kind in KINDS {
        assert!(!new.styles(Role::Message(kind)), "{kind:?}");
        assert!(!with_input.styles(Role::Message(kind)), "{kind:?}");
    }
}

#[test]
fn a_borrowed_or_buffering_sink_keeps_the_wants_styles_of_the_sink_it_borrows() {
    let mut buffer = Vec::new();
    let mut sink = Sink::new(Vec::new(), Vec::new()).styled_errors(true);
    let refusal = Role::Message(Message::Refusal);

    assert!(sink.by_ref().styles(refusal));
    assert!(sink.buffering_result(&mut buffer).styles(refusal));
}

/// The colour variables of an environment that sets none of them.
const UNSET: ColorVariables = ColorVariables {
    no_color: false,
    clicolor_force: false,
    clicolor_off: false,
    term_dumb: false,
};

/// The colour variables with `CLICOLOR_FORCE` forcing colour.
const FORCED: ColorVariables = ColorVariables {
    clicolor_force: true,
    ..UNSET
};

#[test]
fn the_standard_sink_wants_no_wants_styles_under_no_color_even_with_clicolor_force() {
    for terminal in [true, false] {
        for variables in [
            ColorVariables {
                no_color: true,
                ..FORCED
            },
            ColorVariables {
                no_color: true,
                ..UNSET
            },
        ] {
            assert!(!wants_styles(variables, terminal, ColorChoice::Auto, true));
        }
    }
}

#[test]
fn the_standard_sink_wants_wants_styles_with_clicolor_force_when_standard_error_is_not_a_terminal()
{
    assert!(wants_styles(FORCED, false, ColorChoice::Auto, true));
}

#[test]
fn the_standard_sink_wants_no_wants_styles_when_standard_error_is_not_a_terminal_and_clicolor_force_is_unset()
 {
    assert!(!wants_styles(UNSET, false, ColorChoice::Auto, true));
}

#[test]
fn the_standard_sink_wants_wants_styles_when_standard_error_is_a_terminal() {
    assert!(wants_styles(UNSET, true, ColorChoice::Auto, true));
    assert!(wants_styles(UNSET, true, ColorChoice::Always, true));
}

#[test]
fn the_standard_sink_wants_no_wants_styles_at_a_terminal_whose_term_is_dumb() {
    let dumb = ColorVariables {
        term_dumb: true,
        ..UNSET
    };
    assert!(!wants_styles(dumb, true, ColorChoice::Auto, true));
    assert!(!wants_styles(dumb, true, ColorChoice::Always, true));
}

#[test]
fn the_standard_sink_wants_no_wants_styles_at_a_terminal_under_clicolor_0() {
    let off = ColorVariables {
        clicolor_off: true,
        ..UNSET
    };
    assert!(!wants_styles(off, true, ColorChoice::Auto, true));
    assert!(!wants_styles(off, true, ColorChoice::Always, true));
}

#[test]
fn clicolor_force_styles_a_refusal_whose_term_is_dumb_as_anstream_colours_help() {
    let forced = ColorVariables {
        term_dumb: true,
        ..FORCED
    };
    for terminal in [true, false] {
        assert!(wants_styles(forced, terminal, ColorChoice::Auto, true));
    }
}

#[test]
fn clicolor_force_styles_a_refusal_under_clicolor_0_as_anstream_colours_help() {
    let forced = ColorVariables {
        clicolor_off: true,
        ..FORCED
    };
    for terminal in [true, false] {
        assert!(wants_styles(forced, terminal, ColorChoice::Auto, true));
    }
}

#[test]
fn no_color_keeps_a_refusal_plain_under_clicolor_force_whatever_term_and_clicolor_say() {
    let variables = ColorVariables {
        no_color: true,
        clicolor_off: true,
        term_dumb: true,
        ..FORCED
    };
    assert!(!wants_styles(variables, true, ColorChoice::Auto, true));
}

#[test]
fn the_standard_sink_wants_no_wants_styles_when_the_command_never_colours() {
    for (variables, terminal) in [(UNSET, true), (FORCED, false), (FORCED, true)] {
        assert!(!wants_styles(variables, terminal, ColorChoice::Never, true));
    }
}

#[test]
fn the_standard_sink_wants_no_wants_styles_when_the_guide_turns_terminal_detection_off() {
    for (variables, terminal) in [(UNSET, true), (FORCED, false), (FORCED, true)] {
        assert!(!wants_styles(variables, terminal, ColorChoice::Auto, false));
    }
}

#[test]
fn no_color_turns_styles_off_only_with_a_value_that_is_not_empty() {
    assert!(color_off(Some(OsStr::new("1"))));
    assert!(color_off(Some(OsStr::new("0"))));
    assert!(!color_off(Some(OsStr::new(""))));
    assert!(!color_off(None));
}

#[test]
fn clicolor_force_forces_styles_only_with_a_value_that_is_neither_empty_nor_0() {
    assert!(color_forced(Some(OsStr::new("1"))));
    assert!(color_forced(Some(OsStr::new("yes"))));
    assert!(!color_forced(Some(OsStr::new("0"))));
    assert!(!color_forced(Some(OsStr::new(""))));
    assert!(!color_forced(None));
}

#[test]
fn clicolor_turns_styles_off_only_with_the_value_0() {
    assert!(clicolor_off(Some(OsStr::new("0"))));
    assert!(!clicolor_off(Some(OsStr::new("1"))));
    assert!(!clicolor_off(Some(OsStr::new(""))));
    assert!(!clicolor_off(None));
}

#[test]
fn term_turns_styles_off_only_with_the_value_dumb() {
    assert!(term_dumb(Some(OsStr::new("dumb"))));
    assert!(!term_dumb(Some(OsStr::new("xterm-256color"))));
    assert!(!term_dumb(Some(OsStr::new("DUMB"))));
    assert!(!term_dumb(Some(OsStr::new(""))));
    assert!(!term_dumb(None));
}

#[test]
fn only_the_result_has_a_width_on_a_fitted_sink() {
    let fitted = Sink::new(Vec::new(), Vec::new()).fitted(Some(40));
    let unfitted = Sink::new(Vec::new(), Vec::new());

    assert_eq!(fitted.width(Role::Result), Some(40));
    assert_eq!(unfitted.width(Role::Result), None);
    for kind in KINDS {
        assert_eq!(fitted.width(Role::Message(kind)), None, "{kind:?}");
    }
}

#[test]
fn a_sink_from_new_or_with_input_has_no_width() {
    let new = Sink::new(Vec::new(), Vec::new()).styled(true);
    let with_input = Sink::with_input(Vec::new(), Vec::new(), &b""[..], true);

    assert_eq!(new.fitted_width(), None);
    assert_eq!(with_input.fitted_width(), None);
}

#[test]
fn the_width_that_fitted_sets_is_the_width_that_fitted_width_returns() {
    let sink = Sink::new(Vec::new(), Vec::new()).fitted(Some(40));
    assert_eq!(sink.fitted_width(), Some(40));
    assert_eq!(sink.width(Role::Result), Some(40));

    let sink = sink.fitted(Some(0));
    assert_eq!(sink.fitted_width(), Some(0));

    let sink = sink.fitted(None);
    assert_eq!(sink.fitted_width(), None);
}

#[test]
fn a_borrowed_or_buffering_sink_keeps_the_width_of_the_sink_it_borrows() {
    let mut buffer = Vec::new();
    let mut sink = Sink::new(Vec::new(), Vec::new()).fitted(Some(40));

    assert_eq!(sink.by_ref().fitted_width(), Some(40));
    assert_eq!(sink.buffering_result(&mut buffer).fitted_width(), Some(40));
}

#[test]
fn a_sink_over_a_buffer_sends_the_result_to_the_buffer_and_messages_to_the_error_writer() {
    let (mut out, mut err, mut buffer) = (Vec::new(), Vec::new(), Vec::new());
    let mut sink = Sink::new(&mut out, &mut err);

    let mut buffering = sink.buffering_result(&mut buffer);
    buffering.deliver(Role::Result, b"{}\n").unwrap();
    buffering
        .deliver(Role::Message(Message::Refusal), b"error: no\n")
        .unwrap();

    assert_eq!(
        (out, err, buffer),
        (Vec::new(), b"error: no\n".to_vec(), b"{}\n".to_vec())
    );
}

#[test]
fn a_borrowed_sink_writes_to_the_writers_of_the_sink_it_borrows() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut sink = Sink::new(&mut out, &mut err);

    let mut borrowed = sink.by_ref();
    borrowed.deliver(Role::Result, b"rows\n").unwrap();
    borrowed
        .deliver(Role::Message(Message::Note), b"note\n")
        .unwrap();

    assert_eq!((out, err), (b"rows\n".to_vec(), b"note\n".to_vec()));
}

#[test]
fn the_prompt_writer_is_the_error_writer() {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut sink = Sink::with_input(&mut out, &mut err, &b"y\n"[..], true);

    let (input, prompt) = sink.dialogue(Role::Message(Message::Prompt));
    write!(prompt, "Remove? [y/N] ").unwrap();
    let mut answer = String::new();
    io::BufRead::read_line(input, &mut answer).unwrap();

    assert!(sink.is_interactive());
    assert_eq!(answer, "y\n");
    assert_eq!((out, err), (Vec::new(), b"Remove? [y/N] ".to_vec()));
}

/// A writer that takes `room` bytes and then takes none.
struct Short {
    room: usize,
}

impl io::Write for Short {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let taken = buf.len().min(self.room);
        self.room -= taken;
        Ok(taken)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
