//! Property tests for the segment rule (ADR-0006).

use proptest::prelude::*;

use super::is_segment;

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

proptest! {
    #![proptest_config(crate::property::config())]

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
}
