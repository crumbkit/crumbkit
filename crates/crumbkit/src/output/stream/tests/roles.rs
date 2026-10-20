//! Tests for how a stream delivers each line and each message to its sink
//! as one piece (ADR-0128).

use std::io::{self, Write};
use std::ops::ControlFlow;

use super::{SIGHTINGS, Unreadable, kingfisher, parse, robin};
use crate::{Sink, stream_to};

/// A writer that counts its calls, so a test sees how many writes each
/// piece takes.
#[derive(Default)]
struct Calls {
    writes: usize,
    bytes: Vec<u8>,
}

impl Write for Calls {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.writes += 1;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Streams a robin and a kingfisher in text with a failure between them,
/// and returns the two writers.
fn streamed() -> (Calls, Calls) {
    let mut out = Calls::default();
    let mut err = Calls::default();
    let mut sink = Sink::new(&mut out, &mut err);
    let mut streaming = stream_to(&mut sink, parse(&[]), SIGHTINGS).unwrap();
    assert_eq!(streaming.item(&robin()), ControlFlow::Continue(()));
    assert_eq!(streaming.failure(&Unreadable(3)), ControlFlow::Continue(()));
    assert_eq!(streaming.item(&kingfisher()), ControlFlow::Continue(()));
    let _ = streaming.end(Ok::<(), Unreadable>(()));
    (out, err)
}

#[test]
fn each_line_of_a_stream_reaches_the_output_writer_in_one_write() {
    let (out, _) = streamed();

    assert_eq!(out.writes, 3, "{}", String::from_utf8_lossy(&out.bytes));
}

#[test]
fn a_failure_in_a_text_stream_reaches_the_error_writer_in_one_write() {
    let (_, err) = streamed();

    assert_eq!(err.writes, 1, "{}", String::from_utf8_lossy(&err.bytes));
    assert!(err.bytes.starts_with(b"error: line 3"));
}
