//! The helper that the code of `#[derive(Row)]` calls to splice the columns
//! of a flattened field into the columns of its row (ADR-0247).

#[cfg(test)]
mod tests;

/// Returns the names of `parts`, in order, as an array of `N` names.
///
/// The derive calls it in a constant to write a column list that holds the
/// columns of a flattened field's type. The constant's length is the sum of
/// the lengths of the parts, so a call whose `N` differs from that sum is a
/// compile error, because a panic while the compiler evaluates a constant
/// ends the build.
#[doc(hidden)]
#[must_use]
pub const fn __row_columns<const N: usize>(parts: &[&[&'static str]]) -> [&'static str; N] {
    let mut joined = [""; N];
    let mut at = 0;
    let mut rest = parts;
    while let [names, after @ ..] = rest {
        let mut names = *names;
        while let [name, others @ ..] = names {
            assert!(
                at < N,
                "the parts of __row_columns hold more names than its length"
            );
            joined[at] = *name;
            at += 1;
            names = others;
        }
        rest = after;
    }
    assert!(
        at == N,
        "the parts of __row_columns hold fewer names than its length"
    );
    joined
}
