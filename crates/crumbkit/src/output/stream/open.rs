//! How a stream opens: it refuses a view flag that it cannot use, and
//! prints the table's header in text.

use std::borrow::Cow;
use std::io::Write;
use std::process::ExitCode;

use super::{State, Streaming};
use crate::output::{Output, fields, lines, table};
use crate::{Document, Row, Spelling, Stream};

/// Opens the stream on `out` and `err`, as [`stream_to`](super::stream_to)
/// describes.
pub(super) fn open<R: Row, O: Write, W: Write>(
    out: O,
    err: W,
    output: &mut Output,
    spelling: Spelling,
    document: Document<Stream<R>>,
) -> Result<Streaming<'_, R, O, W>, ExitCode> {
    crate::output::columns::require_for_stream::<R>();
    let typed = output.typed();
    let budget = output.budget();
    let printing = output.printing();
    let lined = lines::chosen::<R>(output);
    let Output {
        json,
        fields,
        long,
        full,
        command,
        dry_run,
        ..
    } = output;
    let columns: Vec<&str> = if fields.is_empty() {
        let long = if *long { R::LONG_COLUMNS } else { &[] };
        R::COLUMNS.iter().chain(long).copied().collect()
    } else {
        fields.iter().map(String::as_str).collect()
    };
    let mut streaming = Streaming {
        out,
        err,
        json: *json,
        // A row type that prints whole holds every field in JSON unless
        // `--fields` names some.
        full: *full || (R::WHOLE && fields.is_empty()),
        dry_run: *dry_run,
        lined,
        fields: fields.as_slice(),
        widths: columns.iter().map(|name| name.chars().count()).collect(),
        columns,
        command,
        spelling,
        printing,
        document,
        failed: false,
        exit: 1,
        state: State::Open,
        line: Vec::new(),
        budget,
        sent: 0,
        items: 0,
        problems: 0,
        #[cfg(feature = "tracing")]
        span: crate::trace::spans::finish(),
    };
    let envelope = document.get_envelope().wraps();
    if let Some(refusal) = fields::refusal::<R>(None, envelope, false, streaming.fields, typed) {
        return Err(streaming.refuse(&refusal));
    }
    if !streaming.json && !lined {
        let header: Vec<Cow<'_, str>> = streaming
            .columns
            .iter()
            .map(|name| Cow::Owned(name.to_uppercase()))
            .collect();
        let written = table::write_line(&mut streaming.line, &header, &streaming.widths);
        // The header is written to a `Vec`, which never fails.
        if written.is_ok() {
            let _ = streaming.send();
        }
    }
    Ok(streaming)
}
