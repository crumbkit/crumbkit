//! The character rules for the segments of document ids and failure codes.

#[cfg(test)]
mod tests;

/// Returns whether `text` is a non-empty run of lowercase ASCII letters, ASCII
/// digits and the byte `punctuation`.
///
/// A document id's component and every segment of a failure code allow
/// underscores, and a document id's name allows dashes, so each caller passes
/// the one punctuation byte its segment allows.
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

/// Returns whether `text` is a document id's component: labels joined by
/// single dots, such as `shelf` or `example.dev`, where each label is a
/// non-empty run of lowercase ASCII letters, ASCII digits, `_` and `-`.
///
/// The dots let a component name its owner as a domain does, and a label
/// keeps the underscores that failure codes use and the dashes that domains
/// use.
pub(crate) const fn is_component(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // A `const fn` cannot call iterator methods, so this loop walks the bytes
    // by index. `label` counts the bytes of the current label.
    let mut index = 0;
    let mut label = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'.' {
            if label == 0 {
                return false;
            }
            label = 0;
        } else if byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        {
            label += 1;
        } else {
            return false;
        }
        index += 1;
    }
    label > 0
}
