//! The steps that run their call once with every value of a list field of
//! their failure, as a batch's over-budget failure offers (ADR-0053).

use crate::failures::{ReadFailure, Step};
use crumbkit::FieldValue;

use crate::failures;

/// Returns the list fields of `failure` whose every item fills a
/// placeholder, each a name and the text of its items in order. A list that
/// holds a null, a list or an object is left out, as such a field is when it
/// fills one placeholder.
pub(super) fn lists(failure: &ReadFailure) -> Vec<(String, Vec<String>)> {
    failure
        .fields()
        .iter()
        .filter_map(|(name, value)| {
            let FieldValue::List(items) = value else {
                return None;
            };
            let texts: Option<Vec<String>> = items.iter().map(failures::placeholder_text).collect();
            Some((name.clone(), texts?))
        })
        .collect()
}

/// Returns whether `failure` offers a `RunAll` step whose field holds fewer
/// than `ran` values, so the run that printed it with `ran` values made
/// progress even though it failed with the same code.
pub(super) fn fewer_left(failure: &ReadFailure, ran: usize) -> bool {
    failure.steps().iter().any(|step| {
        let Step::RunAll { field, .. } = step else {
            return false;
        };
        failure.fields().iter().any(|(name, value)| {
            name == field && matches!(value, FieldValue::List(items) if items.len() < ran)
        })
    })
}

/// Returns whether `word` is one placeholder: `<`, one or more ASCII
/// letters, digits, `_` or `-`, and `>`, with clap's `...` after it when the
/// argument takes many values.
pub(super) fn is_placeholder(word: &str) -> bool {
    let word = word.strip_suffix("...").unwrap_or(word);
    word.strip_prefix('<')
        .and_then(|rest| rest.strip_suffix('>'))
        .is_some_and(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}
