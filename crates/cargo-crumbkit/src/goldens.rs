//! The output weight of a command: the size of the largest golden file of
//! its examples, which crumbkit-test's `check_examples` writes.

use std::fs;
use std::io;
use std::path::Path;

use crate::error::WeighError;

#[cfg(test)]
mod tests;

/// Returns the size in bytes of the largest golden file in `dir` of the
/// examples whose command lines are `examples`, or `None` when no example
/// has a golden file there.
///
/// Each example's golden file has the name that crumbkit-test's
/// `example_golden_name` gives its command line, which is the name that
/// `check_examples` writes it under. A missing file counts as no golden
/// file, so a tool whose examples are not pinned has no output weight.
///
/// # Errors
///
/// Returns [`WeighError::GoldenUnreadable`] when a golden file's size
/// cannot be read for a reason other than its absence, such as a `dir` that
/// is a file.
pub(crate) fn largest(dir: &Path, examples: &[String]) -> Result<Option<u64>, WeighError> {
    let mut largest = None;
    for line in examples {
        let path = dir.join(crumbkit_test::example_golden_name(line));
        match fs::metadata(&path) {
            // `None` is less than every `Some`, so the first size wins over
            // no size.
            Ok(metadata) => largest = largest.max(Some(metadata.len())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(WeighError::GoldenUnreadable { path, error }),
        }
    }
    Ok(largest)
}
