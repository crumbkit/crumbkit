//! Helpers that the checks' tests share.

use std::panic::{self, AssertUnwindSafe};

/// Runs `check` and returns its panic message, or `None` when it passes.
///
/// `AssertUnwindSafe` is sound here, because a test drops the command tree
/// after the check panics and never reads it again.
pub(crate) fn panic_message(check: impl FnOnce()) -> Option<String> {
    let payload = panic::catch_unwind(AssertUnwindSafe(check)).err()?;
    Some(
        payload
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default(),
    )
}
