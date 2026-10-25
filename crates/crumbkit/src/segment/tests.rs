//! Property tests for the segment rule, the code segment rule and the
//! component rule (ADR-0006).

use proptest::prelude::*;

use super::{is_code_segment, is_component, is_segment};

/// Generates strings of up to eight characters drawn from the characters that
/// a segment allows and the characters next to them: the upper-case letters,
/// the ASCII neighbours of the letter and digit ranges (`` ` ``, `{`, `/` and
/// `:`), the neighbours of `_` and `-` (`^`, `,` and `.`), a space, NUL, and
/// the letters `é` and `ß` outside ASCII. Some of these strings are segments
/// and most are not, so the test sees both outcomes.
const NEAR_SEGMENTS: &str = r"[a-z0-9_\-A-Z`{/:^,. \x00éß]{0,8}";

/// Returns whether `text` follows the segment rule, stated over characters:
/// it is not empty, and each character is a lowercase ASCII letter, an ASCII
/// digit or `punctuation`. `is_segment` walks bytes instead, so this states
/// the same rule a second way.
fn follows_the_rule(text: &str, punctuation: u8) -> bool {
    !text.is_empty()
        && text.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == char::from(punctuation)
        })
}

/// Returns whether `text` follows the component rule, stated over the
/// labels that `split` returns: every label is non-empty, and each of its
/// characters is a lowercase ASCII letter, an ASCII digit, `_` or `-`.
/// `is_component` walks bytes instead, so this states the same rule a second
/// way.
fn follows_the_component_rule(text: &str) -> bool {
    text.split('.').all(|label| {
        !label.is_empty()
            && label.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || character == '_'
                    || character == '-'
            })
    })
}

/// Returns whether `text` follows the code segment rule, stated over the
/// words that splitting at `-` returns: there is at least one word, every
/// word is non-empty, and each of its characters is a lowercase ASCII
/// letter, an ASCII digit or `_`. A word is empty exactly when a hyphen
/// leads, trails or follows another hyphen. `is_code_segment` walks bytes
/// instead, so this states the same rule a second way.
fn follows_the_code_segment_rule(text: &str) -> bool {
    text.split('-').all(|word| follows_the_rule(word, b'_'))
}

#[test]
fn a_hyphen_between_two_words_is_a_code_segment() {
    assert!(is_code_segment("book-club"));
    assert!(is_code_segment("v2-to-v3"));
    assert!(is_code_segment("not_found-2"));
    assert!(is_code_segment("a_-_b"));
}

#[test]
fn a_leading_trailing_or_doubled_hyphen_is_no_code_segment() {
    assert!(!is_code_segment("-books"));
    assert!(!is_code_segment("books-"));
    assert!(!is_code_segment("book--club"));
    assert!(!is_code_segment("-"));
}

proptest! {
    #[test]
    fn lowercase_letters_digits_and_underscores_are_a_segment_with_underscore_punctuation(
        text in "[a-z0-9_]+",
    ) {
        prop_assert!(is_segment(&text, b'_'));
    }

    #[test]
    fn lowercase_letters_digits_and_dashes_are_a_segment_with_dash_punctuation(
        text in "[a-z0-9-]+",
    ) {
        prop_assert!(is_segment(&text, b'-'));
    }

    #[test]
    fn any_string_is_a_segment_exactly_when_it_follows_the_rule(
        text in prop_oneof![NEAR_SEGMENTS, any::<String>()],
        punctuation in prop_oneof![Just(b'_'), Just(b'-')],
    ) {
        prop_assert_eq!(
            is_segment(&text, punctuation),
            follows_the_rule(&text, punctuation)
        );
    }

    #[test]
    fn labels_joined_by_single_dots_are_a_component(
        text in r"[a-z0-9_-]{1,6}(\.[a-z0-9_-]{1,6}){0,3}",
    ) {
        prop_assert!(is_component(&text));
    }

    #[test]
    fn any_string_is_a_component_exactly_when_it_follows_the_rule(
        text in prop_oneof![NEAR_SEGMENTS, any::<String>()],
    ) {
        prop_assert_eq!(is_component(&text), follows_the_component_rule(&text));
    }

    #[test]
    fn words_of_letters_digits_and_underscores_joined_by_single_hyphens_are_a_code_segment(
        text in "[a-z0-9_]{1,4}(-[a-z0-9_]{1,4}){0,3}",
    ) {
        prop_assert!(is_code_segment(&text));
    }

    #[test]
    fn any_string_is_a_code_segment_exactly_when_it_follows_the_rule(
        text in prop_oneof![NEAR_SEGMENTS, any::<String>()],
    ) {
        prop_assert_eq!(is_code_segment(&text), follows_the_code_segment_rule(&text));
    }
}
