//! `reading entries stream`, which prints each entry of a list as it reads
//! the list's file.

use std::fs;
use std::io::{BufRead as _, BufReader};
use std::ops::ControlFlow;
use std::path::Path;
use std::process::ExitCode;

use crumbkit::{Document, Output, Spelling, Stream};

use super::{EntriesError, Entry, file};

/// The document that `reading entries stream` prints: each entry of a
/// reading list, one on each line as reading reads it.
pub(crate) const ENTRIES_STREAM: Document<Stream<Entry>> = Document::new(
    "reading",
    "entries-stream",
    1,
    "Each entry of a reading list, printed as reading reads it.",
)
.when_empty("the list has no entries");

/// Prints each entry of the list named `name` in `dir` as a stream, in the
/// format that `output` chooses, and returns the process's exit code.
///
/// It reads the list's file one line at a time and prints each entry as it
/// reads it, so the first entries print before the rest of the file is
/// read. It stops reading when the reader of standard output closes the
/// pipe. A list that does not exist or cannot be read ends the stream with
/// its failure.
pub(crate) fn stream_entries(
    output: &mut Output,
    spelling: Spelling,
    dir: Option<&Path>,
    name: &str,
) -> ExitCode {
    let mut streaming = match crumbkit::stream(output, spelling, ENTRIES_STREAM) {
        Ok(streaming) => streaming,
        Err(code) => return code,
    };
    let result = each_entry(dir, name, |entry| streaming.item(&entry));
    streaming.end(result)
}

/// Passes each entry of the list named `name` in `dir` to `item`, in file
/// order, until `item` returns `ControlFlow::Break` or the file ends.
///
/// # Errors
///
/// Returns `EntriesError::NotFound` when `dir` is `None` or holds no list
/// named `name`, and `EntriesError::Unreadable` when a line of the list's
/// file cannot be read as UTF-8 text.
fn each_entry(
    dir: Option<&Path>,
    name: &str,
    mut item: impl FnMut(Entry) -> ControlFlow<()>,
) -> Result<(), EntriesError> {
    let unreadable = |error| EntriesError::Unreadable {
        name: name.to_owned(),
        error,
    };
    let reader = BufReader::new(fs::File::open(file(dir, name)?).map_err(unreadable)?);
    let mut position = 0;
    for line in reader.lines() {
        let line = line.map_err(unreadable)?;
        let title = line.trim();
        if title.is_empty() || title.starts_with('#') {
            continue;
        }
        position += 1;
        let entry = Entry {
            position,
            title: title.to_owned(),
        };
        if item(entry).is_break() {
            break;
        }
    }
    Ok(())
}
