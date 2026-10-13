//! The batch that crumbkit's own tests build from its rows and failures, as
//! a handler builds one from its keys.

use crate::{AsFailure, Batch, Row};

/// Returns a batch of `show` whose argument `names` took one key for each
/// row and then one for each failure, numbered from 1, with every row and
/// then every failure pushed in that order.
pub(crate) fn answered<R: Row, E: AsFailure>(rows: Vec<R>, failures: Vec<E>) -> Batch<R, E> {
    let keys: Vec<usize> = (1..=rows.len() + failures.len()).collect();
    let mut batch = Batch::new(&["show"], "names", &keys);
    batch.extend(rows.into_iter().map(Ok));
    batch.extend(failures.into_iter().map(Err));
    batch
}
