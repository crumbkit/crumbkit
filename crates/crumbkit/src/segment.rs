//! The character rule for the segments of document ids.

/// Returns whether `text` is a non-empty run of lowercase ASCII letters, ASCII
/// digits and the byte `punctuation`.
///
/// A document id's component allows underscores and its name allows dashes, so
/// each caller passes the one punctuation byte its segment allows.
pub(crate) const fn is_segment(text: &str, punctuation: u8) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // A `const fn` cannot call iterator methods, so this loop walks the bytes
    // by index.
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == punctuation) {
            return false;
        }
        index += 1;
    }
    true
}
