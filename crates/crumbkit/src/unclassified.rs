//! An error from outside crumbkit's model, kept as a failure with the
//! unchecked code.

use std::fmt;

use crate::codes::{NO_ADVICE, REPORT_UNCHECKED};
use crate::{Code, Error, Failure, Recovery};

impl Error {
    /// Returns an `Error` that holds an error the tool did not classify, such
    /// as an `io::Error`, an error from another library or a message string.
    ///
    /// The failure's code is `<tool>::report::unchecked` in the tool's
    /// spelling, and its recovery is [`Recovery::None`], because crumbkit
    /// cannot know what the caller should do next. The message is the
    /// error's `Display`, and each error in its `source()` chain prints as a
    /// cause.
    ///
    /// A tool that uses an error library such as anyhow can carry its own
    /// failure types inside that library's error. The tool takes its own type
    /// back out with `downcast` first, so the caller sees the tool's own code,
    /// and passes the rest here. Converting the library's error into a
    /// `Box<dyn Error>` first hides the type inside, so the downcast has to
    /// come before this call. With anyhow the pattern is:
    ///
    /// ```ignore
    /// fn classify(error: anyhow::Error) -> crumbkit::Error {
    ///     match error.downcast::<BooksError>() {
    ///         Ok(failure) => failure.into(),
    ///         Err(error) => crumbkit::Error::unclassified(error),
    ///     }
    /// }
    /// ```
    ///
    /// The same pattern with a `Box<dyn Error>` runs as a test here:
    ///
    /// ```
    /// # use std::fmt;
    /// # use crumbkit::{Code, Failure, Recovery};
    /// # const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
    /// # #[derive(Debug)]
    /// # struct NoSuchBook(u32);
    /// # impl fmt::Display for NoSuchBook {
    /// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    /// #         write!(f, "the catalogue has no book {}", self.0)
    /// #     }
    /// # }
    /// # impl std::error::Error for NoSuchBook {}
    /// # impl Failure for NoSuchBook {
    /// #     fn code(&self) -> &'static Code { &NOT_FOUND }
    /// #     fn recovery(&self) -> Recovery { Recovery::Retry }
    /// # }
    /// use crumbkit::codes::REPORT_UNCHECKED;
    /// use crumbkit::Error;
    ///
    /// type AnyError = Box<dyn std::error::Error + Send + Sync>;
    ///
    /// fn classify(error: AnyError) -> Error {
    ///     match error.downcast::<NoSuchBook>() {
    ///         Ok(failure) => Error::from(*failure),
    ///         Err(error) => Error::unclassified(error),
    ///     }
    /// }
    ///
    /// let known = classify(Box::new(NoSuchBook(7)));
    /// assert_eq!(known.failure().code(), &NOT_FOUND);
    ///
    /// let unknown = classify("the catalogue is locked".into());
    /// assert_eq!(unknown.failure().code(), &REPORT_UNCHECKED);
    /// ```
    pub fn unclassified(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        Self::from(Unclassified(error.into()))
    }
}

/// An error that the tool did not classify, as a [`Failure`].
struct Unclassified(Box<dyn std::error::Error + Send + Sync>);

/// Prints the error's `Debug`.
impl fmt::Debug for Unclassified {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// Prints the error's `Display`.
impl fmt::Display for Unclassified {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Returns the error's `source()`, so each error in its chain prints as a
/// cause.
impl std::error::Error for Unclassified {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl Failure for Unclassified {
    fn code(&self) -> &'static Code {
        &REPORT_UNCHECKED
    }

    fn recovery(&self) -> Recovery {
        Recovery::None {
            reason: NO_ADVICE.to_owned(),
        }
    }
}

#[cfg(test)]
mod tests;
