//! The tool record check of the notes that `Leaf::notes` sets.

use crumbkit::help::Leaf;

/// Adds a problem to `problems` for each note of the leaf at `text` that is
/// empty or holds a line break, because help prints each note on one
/// indented line under `Notes:`.
pub(super) fn check_notes(text: &str, leaf: &Leaf, problems: &mut Vec<String>) {
    for note in leaf.get_notes() {
        if note.trim().is_empty() {
            problems.push(format!(
                "{text}: a note is empty, so help prints a blank line under `Notes:`; remove it"
            ));
        } else if note.contains(['\n', '\r']) {
            problems.push(format!(
                "{text}: the note `{}` holds a line break, so help cannot print it on one indented line; split it into one note for each line",
                note.escape_debug()
            ));
        }
    }
}
