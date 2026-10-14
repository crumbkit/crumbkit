//! The strict parser of a W3C `traceparent` value, which the caller passes
//! in the `TRACEPARENT` environment variable (ADR-0030).

#[cfg(test)]
mod tests;

/// The fields of a valid `traceparent` value that a run keeps.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Traceparent {
    /// The trace id, which is never all zero.
    pub(super) trace: [u8; 16],
    /// The caller's span id, which is never all zero.
    pub(super) parent: [u8; 8],
    /// The trace flags, every bit as the caller sent it.
    pub(super) flags: u8,
}

/// Returns the fields of `value` when it has the shape of W3C Trace Context
/// Level 1, or `None` otherwise.
///
/// The shape is a version of two lowercase hex digits, a trace id of 32, a
/// parent id of 16 and flags of 2, joined by single dashes. Version `ff` is
/// invalid, and a version `00` value has exactly those four fields. A higher
/// version may add fields after another dash, which the parser ignores. A
/// trace id or a parent id that is all zero is invalid, and so is an
/// uppercase hex digit.
pub(super) fn parse(value: &str) -> Option<Traceparent> {
    let mut fields = value.split('-');
    let [version] = decode::<1>(fields.next()?)?;
    let trace: [u8; 16] = decode(fields.next()?)?;
    let parent: [u8; 8] = decode(fields.next()?)?;
    let [flags] = decode::<1>(fields.next()?)?;
    let extra = fields.next().is_some();
    if version == 0xff || version == 0x00 && extra {
        return None;
    }
    if trace == [0; 16] || parent == [0; 8] {
        return None;
    }
    Some(Traceparent {
        trace,
        parent,
        flags,
    })
}

/// Returns the `N` bytes that `text` spells as exactly `2 * N` lowercase hex
/// digits, or `None` when it spells anything else.
fn decode<const N: usize>(text: &str) -> Option<[u8; N]> {
    let digits = text.as_bytes();
    if digits.len() != 2 * N {
        return None;
    }
    let (pairs, _) = digits.as_chunks::<2>();
    let mut bytes = [0; N];
    for (byte, [high, low]) in bytes.iter_mut().zip(pairs) {
        *byte = digit(*high)? * 16 + digit(*low)?;
    }
    Some(bytes)
}

/// Returns the value of the lowercase hex digit `byte`, or `None` for any
/// other byte, including an uppercase digit.
const fn digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
