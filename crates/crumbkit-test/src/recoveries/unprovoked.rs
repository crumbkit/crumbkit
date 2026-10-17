//! The declared codes that no command line in a test can provoke.

use crumbkit::Code;

use crate::allow::is_reason;

/// A declared code that no command line in a test can provoke, with the
/// reason, such as a failure that needs a full disk. [`check_recoveries`](crate::check_recoveries)
/// starts no chain for it, and [`check_twins`](crate::check_twins) needs no
/// twin for it, so a tool passes the same list to both.
#[derive(Clone, Copy, Debug)]
pub struct Unprovoked<'a> {
    /// The declared code.
    code: Code,
    /// Why no test can provoke it, in one line.
    reason: &'a str,
}

impl<'a> Unprovoked<'a> {
    /// Returns the entry that lists `code` as unprovoked for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A
    /// `const` that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit::Code;
    /// use crumbkit_test::Unprovoked;
    ///
    /// const GONE: Code = Code::new("shelf", "books", "removed_file");
    /// const BAD: Unprovoked<'_> = Unprovoked::new(&GONE, "");
    /// ```
    #[must_use]
    pub const fn new(code: &'a Code, reason: &'a str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            code: *code,
            reason,
        }
    }

    /// Returns the declared code.
    pub(crate) const fn code(&self) -> &Code {
        &self.code
    }

    /// Returns why no test can provoke the code.
    pub(crate) const fn reason(&self) -> &'a str {
        self.reason
    }
}
