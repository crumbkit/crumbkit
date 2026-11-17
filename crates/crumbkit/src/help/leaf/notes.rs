//! The `Notes:` section of a leaf's help, and the block that joins it to
//! the examples.

use clap::builder::styling::Style;

use super::Leaf;
use super::example::examples;

/// Returns the `Notes:` section of the help of `leaf`, or `None` when the
/// leaf has no notes. `header` styles the heading. Each note prints on one
/// line indented two spaces, as an example line is.
fn notes(leaf: &Leaf, header: Style) -> Option<String> {
    let notes = leaf.get_notes();
    if notes.is_empty() {
        return None;
    }
    let mut section = format!("{header}Notes:{header:#}");
    for note in notes {
        section.push_str("\n  ");
        section.push_str(note);
    }
    Some(section)
}

/// Returns the block that `-h`, or `--help` when `long` is set, prints for
/// `leaf` after the command's options: its `Examples:` section and then its
/// `Notes:` section, with a blank line between them, or `None` when the
/// leaf has neither. `text` is the command line that starts each example.
///
/// `parse` and [`extend`](crate::help::extend) both build the block here,
/// so a command that `extend` wrote it to compares equal to the block
/// `parse` builds, and it prints once.
pub(crate) fn leaf_text(
    leaf: &Leaf,
    text: &str,
    long: bool,
    header: Style,
    literal: Style,
) -> Option<String> {
    let examples = examples(leaf, text, long, header, literal);
    let notes = notes(leaf, header);
    match (examples, notes) {
        (Some(examples), Some(notes)) => Some(format!("{examples}\n\n{notes}")),
        (examples, notes) => examples.or(notes),
    }
}
