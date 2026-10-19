//! The conversion from a `miette::Report` into an `Error`, which the
//! `miette` feature adds.

use std::fmt;

use crate::codes::REPORT_UNCHECKED;
use crate::{Code, Error, Failure, Recovery};

/// Converts a `miette::Report` into an `Error`, so `?` works on a
/// `miette::Result` inside a handler that returns `Result<_, Error>`. The
/// conversion exists only with the `miette` feature, which crumbkit's own
/// `miette` feature turns on.
///
/// The failure's code is `<tool>::report::unchecked` in the tool's
/// spelling for every report, because a report's own code is free text that
/// no codes check has seen. The message is the report's `Display`, followed
/// by the report's own code in parentheses when it has one, such as
/// `unknown key (shelf::config::unknown_key)`. The report's help becomes a
/// [`Recovery::Hint`], and a report without help has a [`Recovery::None`].
/// The url is the report's url, each error in the report's chain prints as a
/// cause, and the labels of the report's diagnostic print as a failure's
/// labels do.
///
/// The fixed code tells the caller only that the tool did not classify the
/// failure. A tool whose own failure type travels inside a report takes it
/// back out with `downcast` before converting, so the caller sees the tool's
/// own code:
///
/// ```
/// # use std::fmt;
/// # use crumbkit_core::{Code, Failure, Recovery};
/// # const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
/// # #[derive(Debug)]
/// # struct NoSuchBook(u32);
/// # impl fmt::Display for NoSuchBook {
/// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
/// #         write!(f, "the catalogue has no book {}", self.0)
/// #     }
/// # }
/// # impl std::error::Error for NoSuchBook {}
/// # impl miette::Diagnostic for NoSuchBook {}
/// # impl Failure for NoSuchBook {
/// #     fn code(&self) -> &'static Code { &NOT_FOUND }
/// #     fn recovery(&self) -> Recovery { Recovery::Retry }
/// # }
/// fn classify(report: miette::Report) -> crumbkit_core::Error {
///     match report.downcast::<NoSuchBook>() {
///         Ok(failure) => failure.into(),
///         Err(report) => report.into(),
///     }
/// }
///
/// let error = classify(miette::Report::new(NoSuchBook(7)));
/// assert_eq!(error.failure().code(), &NOT_FOUND);
/// ```
//
// This impl and the blanket `impl<E: Failure + Send + Sync + 'static>
// From<E> for Error` do not overlap, because `miette::Report` does not
// implement `Failure`. `Failure` is crumbkit's trait and `Report` is miette's
// type, so the orphan rule lets only crumbkit implement the one for the
// other, and the compiler can rely on crumbkit not doing so.
impl From<miette::Report> for Error {
    fn from(report: miette::Report) -> Self {
        let url = report.url().map(|url| url.to_string());
        Self::from(ReportFailure { report, url })
    }
}

/// A `miette::Report` as a [`Failure`].
#[derive(Debug)]
struct ReportFailure {
    /// The report.
    report: miette::Report,
    /// The report's url as text, because `Failure::url` returns a `&str` and
    /// the report gives its url as a value that implements `Display`.
    url: Option<String>,
}

/// Prints the report's `Display`, then the report's own code in parentheses
/// when it has one.
impl fmt::Display for ReportFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.report, f)?;
        match self.report.code() {
            Some(code) => write!(f, " ({code})"),
            None => Ok(()),
        }
    }
}

/// Returns the `source()` of the report, so each error in its chain prints
/// as a cause.
impl std::error::Error for ReportFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.report.source()
    }
}

impl Failure for ReportFailure {
    fn code(&self) -> &'static Code {
        &REPORT_UNCHECKED
    }

    fn recovery(&self) -> Recovery {
        match self.report.help() {
            Some(help) => Recovery::Hint(help.to_string()),
            None => Recovery::no_advice(),
        }
    }

    fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    fn diagnostic(&self) -> Option<&dyn miette::Diagnostic> {
        Some(&*self.report)
    }
}
