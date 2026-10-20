use std::io;

use super::Sink;
use super::role::{Deliver as _, Message, Role};

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
