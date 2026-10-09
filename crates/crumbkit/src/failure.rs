//! The failure model: the `Failure` trait, its `Recovery`, and `Error`, which
//! holds any failure.

use std::fmt;

use crate::{Call, Code};

#[cfg(test)]
mod tests;

/// An error that a command reports to its caller with a stable code and a
/// next step.
///
/// A tool implements it beside `std::error::Error` for each of its error
/// types. [`finish`](crate::finish) prints the error's `Display` as the
/// failure's message, each error in its `source()` chain as a cause, the
/// recovery, the code in the tool's [`Spelling`](crate::Spelling), and the
/// url when there is one.
///
/// ```
/// use std::fmt;
///
/// use crumbkit::{Call, Code, Failure, Recovery};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
///
/// #[derive(Debug)]
/// struct NoSuchBook(u32);
///
/// impl fmt::Display for NoSuchBook {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "the catalogue has no book {}", self.0)
///     }
/// }
///
/// impl std::error::Error for NoSuchBook {}
///
/// impl Failure for NoSuchBook {
///     fn code(&self) -> &'static Code {
///         &NOT_FOUND
///     }
///
///     fn recovery(&self) -> Recovery {
///         Recovery::Run(Call::new(&["books", "list"]))
///     }
/// }
/// ```
pub trait Failure: std::error::Error {
    /// Returns the failure's code. Each error type lists the codes it
    /// returns in a `CODES` constant, which the tool's test passes to
    /// crumbkit-test's codes check.
    fn code(&self) -> &'static Code;

    /// Returns what the caller can do next.
    fn recovery(&self) -> Recovery;

    /// Returns a page that explains the failure, or `None`, the default,
    /// when there is none.
    fn url(&self) -> Option<&str> {
        None
    }

    /// Returns the failure as a `miette::Diagnostic` whose labels point into
    /// a source, such as a settings file, or `None`, the default. The method
    /// exists only with crumbkit's `miette` feature.
    ///
    /// A failure type that also implements `miette::Diagnostic` returns
    /// `Some(self)`. [`finish`](crate::finish) then reads the diagnostic's
    /// `labels()` against its `source_code()`, and prints each label as an
    /// `at:` line with the source line and an underline in text, and under
    /// `labels` in JSON. crumbkit reads nothing else from the diagnostic, so
    /// the code, the recovery and the url still come from this trait.
    ///
    /// ```
    /// # use std::fmt;
    /// # use crumbkit::{Code, Failure, Recovery};
    /// # const UNKNOWN_KEY: Code = Code::new("shelf", "config", "unknown_key");
    /// # #[derive(Debug)]
    /// # struct UnknownKey { source: String, at: usize }
    /// # impl fmt::Display for UnknownKey {
    /// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("unknown key") }
    /// # }
    /// # impl std::error::Error for UnknownKey {}
    /// use miette::{Diagnostic, LabeledSpan, SourceCode};
    ///
    /// impl Diagnostic for UnknownKey {
    ///     fn source_code(&self) -> Option<&dyn SourceCode> {
    ///         Some(&self.source)
    ///     }
    ///
    ///     fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
    ///         let label = LabeledSpan::new(Some("this key is not known".to_owned()), self.at, 6);
    ///         Some(Box::new(std::iter::once(label)))
    ///     }
    /// }
    ///
    /// impl Failure for UnknownKey {
    ///     fn code(&self) -> &'static Code {
    ///         &UNKNOWN_KEY
    ///     }
    ///
    ///     fn recovery(&self) -> Recovery {
    ///         Recovery::Hint("use `color`".to_owned())
    ///     }
    ///
    ///     fn diagnostic(&self) -> Option<&dyn Diagnostic> {
    ///         Some(self)
    ///     }
    /// }
    /// ```
    #[cfg(feature = "miette")]
    fn diagnostic(&self) -> Option<&dyn miette::Diagnostic> {
        None
    }
}

/// What the caller of a failed command can do next. Every failure states
/// one.
///
/// The text output prints it on the `help:` line, and the JSON failure
/// document holds it under `recovery`, tagged by `kind`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Recovery {
    /// Run this command, such as `shelf books list`. [`finish`](crate::finish)
    /// renders the call against the command that
    /// [`Output::set_command`](crate::Output::set_command) received. When no
    /// command is set or the call does not resolve, the failure prints as
    /// `None` with the reason `the suggested command is not available`.
    Run(Call),
    /// Run the same command again later, because the failure may pass.
    Retry,
    /// Follow this advice, such as `check the network connection`.
    Hint(String),
    /// The caller has no next step, for the reason given.
    None {
        /// Why the caller has no next step.
        reason: String,
    },
}

/// Any [`Failure`], so a handler that calls code with different error types
/// can return one type and use `?` on each.
///
/// Every type that implements `Failure`, `Send` and `Sync` converts into an
/// `Error`, and [`finish`](crate::finish) prints it as it prints the failure
/// inside.
///
/// `Error` does not implement `Failure`. If it did, the blanket `From` impl
/// would also cover `From<Error> for Error`, which the standard library
/// already provides for every type, and the compiler rejects the overlap.
/// `finish` accepts an `Error` through [`AsFailure`] instead, and
/// [`Error::failure`] gives read access to the failure inside.
///
/// `Error` implements `std::error::Error`, with the `Display` and `source()`
/// of the failure inside, so a tool's `main` can pass it on to
/// `Box<dyn std::error::Error>` or another error library.
///
/// ```
/// # use std::fmt;
/// # use crumbkit::{Code, Failure, Recovery};
/// # const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");
/// # #[derive(Debug)]
/// # struct Timeout;
/// # impl fmt::Display for Timeout {
/// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("timed out") }
/// # }
/// # impl std::error::Error for Timeout {}
/// # impl Failure for Timeout {
/// #     fn code(&self) -> &'static Code { &TIMED_OUT }
/// #     fn recovery(&self) -> Recovery { Recovery::Retry }
/// # }
/// fn fetch() -> Result<String, Timeout> {
///     Err(Timeout)
/// }
///
/// fn handler() -> Result<String, crumbkit::Error> {
///     let text = fetch()?;
///     Ok(text)
/// }
///
/// let error = handler().unwrap_err();
/// assert_eq!(error.to_string(), "timed out");
/// assert_eq!(error.failure().recovery(), Recovery::Retry);
/// ```
pub struct Error(Box<dyn Failure + Send + Sync>);

impl Error {
    /// Returns the failure inside. It is `'static`, so a caller can upcast it
    /// to `&dyn std::error::Error` and downcast it to its own type.
    #[must_use]
    pub fn failure(&self) -> &(dyn Failure + Send + Sync + 'static) {
        &*self.0
    }
}

impl<E: Failure + Send + Sync + 'static> From<E> for Error {
    fn from(failure: E) -> Self {
        Self(Box::new(failure))
    }
}

/// Prints the failure's `Debug`.
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

/// Prints the failure's `Display`.
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Returns the `source()` of the failure inside.
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// A value that [`finish`](crate::finish) prints as a failure: any
/// [`Failure`], or an [`Error`].
///
/// `finish` takes a handler's `Result<T, E>` with `E: AsFailure`, so a
/// handler returns either its own error type or an `Error`. The trait exists
/// because `Error` cannot implement `Failure`, and `finish` needs one bound
/// that both satisfy. crumbkit implements it for both, and a tool cannot
/// implement it.
pub trait AsFailure: sealed::Sealed {
    /// Returns the failure to print.
    fn as_failure(&self) -> &dyn Failure;
}

impl<F: Failure> AsFailure for F {
    fn as_failure(&self) -> &dyn Failure {
        self
    }
}

impl AsFailure for Error {
    fn as_failure(&self) -> &dyn Failure {
        self.failure()
    }
}

/// Holds the supertrait that keeps tools from implementing [`AsFailure`].
mod sealed {
    /// A type that crumbkit implements [`AsFailure`](super::AsFailure) for.
    /// The trait is public so it can bound `AsFailure`, and it sits in a
    /// private module so no tool can name it.
    pub trait Sealed {}

    impl<F: super::Failure> Sealed for F {}

    impl Sealed for super::Error {}
}
