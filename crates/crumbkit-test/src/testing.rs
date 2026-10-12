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

/// Writes a `/bin/sh` script named `tool` in `dir`, which stands in for a
/// tool's binary, and returns its path. The script prints the page of each
/// entry of `pages` for the command line of its words, joined by single
/// spaces, such as `books list -h`, and fails every other command line with
/// code 2. A page holds no single quote.
#[cfg(unix)]
pub(crate) fn help_binary(dir: &crate::temp::TempDir, pages: &[(&str, &str)]) -> String {
    use std::fmt::Write as _;

    let mut script = String::from("#!/bin/sh\ncase \"$*\" in\n");
    for (line, page) in pages {
        writeln!(script, "  '{line}') printf '%s' '{page}';;").unwrap();
    }
    script.push_str("  *) exit 2;;\nesac\n");
    dir.write_script("tool", &script);
    dir.path().join("tool").display().to_string()
}
