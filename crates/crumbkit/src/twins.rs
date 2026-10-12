//! The check that crumbkit's own refusal tests use to pair each refused
//! command line with an admitted twin (TEST-1).
//!
//! crumbkit-test's `check_twin` runs a built binary, so a test inside
//! crumbkit checks the pair with [`assert_one_word_apart`] and runs the
//! admitted words through the same code as the refused ones.

#[cfg(test)]
mod tests;

/// Panics unless `refused` and `admitted` differ by one word added, removed
/// or replaced.
//
// RUST-10: crumbkit-test's `check_twin` has the same comparison. crumbkit
// cannot use it, because crumbkit-test depends on crumbkit, and a test of
// crumbkit that linked crumbkit-test would see two copies of crumbkit's
// types. The comparison is ten lines, so each crate keeps its own copy.
#[track_caller]
pub(crate) fn assert_one_word_apart(refused: &[&str], admitted: &[&str]) {
    let (long, short) = if refused.len() >= admitted.len() {
        (refused, admitted)
    } else {
        (admitted, refused)
    };
    let first = long
        .iter()
        .zip(short)
        .position(|(a, b)| a != b)
        .unwrap_or(short.len());
    let apart = match long.len() - short.len() {
        0 => first < long.len() && long[first + 1..] == short[first + 1..],
        1 => long[first + 1..] == short[first..],
        _ => false,
    };
    assert!(
        apart,
        "the refused words {refused:?} and the admitted words {admitted:?} must differ by one word added, removed or replaced"
    );
}
