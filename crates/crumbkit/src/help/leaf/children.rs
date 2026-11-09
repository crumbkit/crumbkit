//! The `Children:` line of a leaf whose rows print their children as lines
//! under them (ADR-0120, ADR-0137).

use super::Leaf;

#[cfg(test)]
mod tests;

/// Returns the `Children:` line that prints `text`, the description of the
/// child lines that
/// [`LeafDocument::children_text`](super::LeafDocument::children_text) returns, such as `Children: one
/// indented line under each row for each of its components.`. Help prints
/// this line, and crumbkit-markdown prints it with the `children_help` text
/// of `commands list`, so both print the same label.
#[must_use]
pub fn children_line(text: &str) -> String {
    format!("Children: {text}")
}

impl Leaf {
    /// Returns the `Children:` line that the leaf's `-h` and `--help` print
    /// after the `Output:` line and the `Text:` line, as [`children_line`]
    /// builds it from the document's child-line text, or `None` when the leaf's
    /// row type names no children field. A headed leaf has none, because
    /// its text prints each row's bytes.
    pub(crate) fn children_help_line(&self) -> Option<String> {
        let document = self.get_document()?;
        let text = document.children_text()?;
        if document.document.is_headed() {
            return None;
        }
        Some(children_line(&text))
    }
}
