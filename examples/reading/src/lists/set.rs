//! `reading entries set`, which replaces one entry of a list or appends one
//! after the last.

use std::fs;
use std::path::Path;

use crumbkit::{Document, Error, Output};

use super::{EntriesError, Entry, every_entry, file, is_entry};
use crate::cli::EntriesSet;

/// The document that `reading entries set` prints: the entry that the run
/// writes, or would write on a dry run.
pub(crate) const ENTRIES_SET: Document<Entry> = Document::new(
    "reading",
    "entries-set",
    1,
    "The entry that the run writes to a reading list, or would write on a dry run.",
);

/// Writes the title that `args` names at its position in the list that it
/// names in `dir`, and returns the entry. A position one past the last
/// entry appends the entry after the last line of the file. On a dry run,
/// which [`Output::dry_run`] records, it returns the same entry and writes
/// nothing.
///
/// The version that `--expect` names is the title that the entry has now,
/// and `--expect none` requires that the position is one past the last
/// entry. The function checks it and then asks for confirmation with
/// [`Output::confirm`], so a conflict comes before the question.
///
/// # Errors
///
/// Returns `EntriesError::NotFound` when `dir` holds no such list,
/// `EntriesError::Unreadable` when its file cannot be read or written,
/// `EntriesError::OutOfRange` when the position is more than one past the
/// last entry, the conflict of `Expect::check`, and the failures of
/// [`Output::confirm`].
//
// RUST-10: a title is a weak version, because two edits can leave an entry
// with the title it had, and a real tool would compare a revision number or
// an ETag that changes on every write. The function also reads the file,
// checks the version and writes the file in three steps without a lock, so
// a write by another process between the check and the write is lost.
pub(crate) fn set(
    output: &mut Output,
    dir: Option<&Path>,
    args: &EntriesSet,
) -> Result<Entry, Error> {
    let dry_run = output.dry_run(args.dry_run);
    let name = args.list.as_str();
    let path = file(dir, name)?;
    let unreadable = |error| EntriesError::Unreadable {
        name: name.to_owned(),
        error,
    };
    let text = fs::read_to_string(&path).map_err(unreadable)?;
    let mut lines: Vec<&str> = text.lines().collect();
    let entries: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| is_entry(line))
        .map(|(index, _)| index)
        .collect();
    let position = args.position.get();
    if position > entries.len() + 1 {
        return Err(EntriesError::OutOfRange {
            name: name.to_owned(),
            count: entries.len(),
        }
        .into());
    }
    let line = entries.get(position - 1).copied();
    let current = line.map(|line| lines[line].trim());
    args.expect.check(current, every_entry(name))?;
    output.confirm(
        args.yes,
        &format!("Write entry {position} of the {name} list?"),
    )?;
    let entry = Entry {
        position,
        title: args.title.clone(),
    };
    if dry_run {
        return Ok(entry);
    }
    match line {
        Some(line) => lines[line] = &args.title,
        None => lines.push(&args.title),
    }
    let mut written = lines.join("\n");
    written.push('\n');
    #[expect(
        clippy::disallowed_methods,
        reason = "this module owns the write of `reading entries set`, reading's only file write"
    )]
    fs::write(&path, written).map_err(unreadable)?;
    Ok(entry)
}
