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
/// types. `crumbkit::finish` prints the error's `Display` as the
/// failure's message, each error in its `source()` chain as a cause, the
/// recovery, the code in the tool's [`Spelling`](crate::Spelling), and the
/// url when there is one.
///
/// ```
/// use std::fmt;
///
/// use crumbkit_core::{Call, Code, Failure, Recovery};
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
///
/// crumbkit reads only the failure that a handler returns. It cannot find a
/// `Failure` in the `source()` chain, because `source()` returns a
/// `&dyn std::error::Error`. A type that wraps a failure to add context
/// therefore implements `Failure` itself and returns the code and the
/// repairs of the failure it wraps:
///
/// ```
/// # use std::fmt;
/// # use crumbkit_core::{Call, Code, Failure, Recovery};
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
/// #     fn recovery(&self) -> Recovery { Recovery::Run(Call::new(&["books", "list"])) }
/// # }
/// /// A failure to read a reading list, caused by the book it names.
/// #[derive(Debug)]
/// struct ReadingList {
///     name: String,
///     book: NoSuchBook,
/// }
///
/// impl fmt::Display for ReadingList {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(f, "cannot read the reading list {}", self.name)
///     }
/// }
///
/// impl std::error::Error for ReadingList {
///     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
///         Some(&self.book)
///     }
/// }
///
/// impl Failure for ReadingList {
///     fn code(&self) -> &'static Code {
///         self.book.code()
///     }
///
///     fn recovery(&self) -> Recovery {
///         self.book.recovery()
///     }
///
///     fn alternatives(&self) -> Vec<Recovery> {
///         self.book.alternatives()
///     }
///
///     fn url(&self) -> Option<&str> {
///         self.book.url()
///     }
/// }
///
/// let failure = ReadingList { name: "summer".to_owned(), book: NoSuchBook(7) };
/// assert_eq!(failure.recovery(), Recovery::Run(Call::new(&["books", "list"])));
/// ```
pub trait Failure: std::error::Error {
    /// Returns the failure's code. Each error type lists the codes it
    /// returns in a `CODES` constant, which the tool's test passes to
    /// crumbkit-test's codes check.
    fn code(&self) -> &'static Code;

    /// Returns what the caller can do next, the preferred repair.
    fn recovery(&self) -> Recovery;

    /// Returns other repairs that the caller can try when the preferred one
    /// does not suit, such as setting a token variable instead of logging
    /// in, in order of preference. The default is an empty list, which does
    /// not allocate.
    ///
    /// `crumbkit::finish` prints each alternative on its own `help:` line
    /// after the recovery's, and under `alternatives` in JSON. It leaves out
    /// an alternative that offers no step: a `Recovery::Run` whose call does
    /// not resolve, and a `Recovery::None`. When the recovery itself offers
    /// no step, the first alternative that offers one prints in its place.
    ///
    /// ```
    /// # use std::fmt;
    /// # use crumbkit_core::{Call, Code, Failure, Recovery};
    /// # const NO_LOGIN: Code = Code::new("shelf", "auth", "no_login");
    /// # #[derive(Debug)]
    /// # struct NoLogin;
    /// # impl fmt::Display for NoLogin {
    /// #     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str("not logged in") }
    /// # }
    /// # impl std::error::Error for NoLogin {}
    /// impl Failure for NoLogin {
    ///     fn code(&self) -> &'static Code {
    ///         &NO_LOGIN
    ///     }
    ///
    ///     fn recovery(&self) -> Recovery {
    ///         Recovery::Run(Call::new(&["login"]))
    ///     }
    ///
    ///     fn alternatives(&self) -> Vec<Recovery> {
    ///         vec![Recovery::Hint("set SHELF_TOKEN to an access token".to_owned())]
    ///     }
    /// }
    /// ```
    fn alternatives(&self) -> Vec<Recovery> {
        Vec::new()
    }

    /// Returns a page that explains the failure, or `None`, the default,
    /// when there is none.
    fn url(&self) -> Option<&str> {
        None
    }

    /// Returns the failure as a `miette::Diagnostic` whose labels point into
    /// a source, such as a settings file, or `None`, the default. The method
    /// exists only with the `miette` feature, which crumbkit's own
    /// `miette` feature turns on.
    ///
    /// A failure type that also implements `miette::Diagnostic` returns
    /// `Some(self)`. `crumbkit::finish` then reads the diagnostic's
    /// `labels()` against its `source_code()`, and prints each label as an
    /// `at:` line with the source line and an underline in text, and under
    /// `labels` in JSON. crumbkit reads nothing else from the diagnostic, so
    /// the code, the recovery and the url still come from this trait.
    ///
    /// ```
    /// # use std::fmt;
    /// # use crumbkit_core::{Code, Failure, Recovery};
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
/// one, and may offer more through [`Failure::alternatives`].
///
/// The text output prints it on the `help:` line, and the JSON failure
/// document holds it under `recovery`, tagged by `kind`, and each
/// alternative under `alternatives` in the same shape.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Recovery {
    /// Run this command, such as `shelf books list`. `crumbkit::finish`
    /// renders the call against the command that
    /// `crumbkit::Output::set_command` received. When no
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
/// `Error`, and `crumbkit::finish` prints it as it prints the failure
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
/// # use crumbkit_core::{Code, Failure, Recovery};
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
/// fn handler() -> Result<String, crumbkit_core::Error> {
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

/// A value that `crumbkit::finish` prints as a failure: any
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
