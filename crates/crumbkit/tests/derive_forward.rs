//! `code = EXPR`, `codes` and the forwarding of a foreign error through a
//! newtype, through crumbkit's `Failure` trait. The newtype test copies the
//! "Forwarding a foreign error" example of
//! docs/behaviour/failures/derive.md, so the documented pattern compiles.

use std::fmt;

use crumbkit::{Code, CodeBuf, Failure, Recovery, Spelling};

const JOB_NOT_FOUND: Code<'static> = Code::new("tool", "job", "not-found");
const JOB_NOT_STARTED: Code<'static> = Code::new("tool", "job", "not-started");
const LIB_TIMEOUT: Code<'static> = Code::new("tool", "lib", "timeout");
const LIB_REFUSED: Code<'static> = Code::new("tool", "lib", "refused");

/// A failure whose codes come from expressions.
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(crate = ::crumbkit)]
enum JobError {
    #[failure(
        code = if *missing { JOB_NOT_FOUND } else { JOB_NOT_STARTED },
        codes = [JOB_NOT_FOUND, JOB_NOT_STARTED],
        hint = "check that `{program}` is an executable file",
    )]
    NotStarted { program: String, missing: bool },
    #[failure(code = spelled.as_code(), codes = [], hint = "use a known code")]
    Spelled { spelled: CodeBuf },
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the job failed")
    }
}

impl std::error::Error for JobError {}

#[test]
fn a_code_expression_chooses_the_code_from_the_fields() {
    let missing = JobError::NotStarted {
        program: "make".into(),
        missing: true,
    };
    let refused = JobError::NotStarted {
        program: "make".into(),
        missing: false,
    };
    assert_eq!(missing.code(), JOB_NOT_FOUND);
    assert_eq!(refused.code(), JOB_NOT_STARTED);
    assert_eq!(JobError::CODES, &[JOB_NOT_FOUND, JOB_NOT_STARTED]);
}

#[test]
fn a_code_expression_can_return_a_code_that_borrows_from_a_field() {
    let spelled = Code::new("tool", "job", "made-up").to_buf();
    let error = JobError::Spelled { spelled };
    assert_eq!(Spelling::COLONS.spell(error.code()), "tool::job::made-up");
}

/// A stand-in for an error from a library with no crumbkit dependency.
#[derive(Debug)]
enum LibKind {
    Timeout,
    Refused,
}

#[derive(Debug)]
struct LibError {
    kind: LibKind,
}

impl fmt::Display for LibError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the library failed: {:?}", self.kind)
    }
}

impl std::error::Error for LibError {}

// The newtype owns the mapping from the library's error to crumbkit's answers.
#[derive(Debug)]
struct LibraryFailure(LibError);

impl fmt::Display for LibraryFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for LibraryFailure {}

impl From<LibError> for LibraryFailure {
    fn from(error: LibError) -> Self {
        Self(error)
    }
}

impl LibraryFailure {
    /// The codes the newtype returns, which the twin and recovery checks read.
    const CODES: &'static [Code<'static>] = &[LIB_TIMEOUT, LIB_REFUSED];
}

impl Failure for LibraryFailure {
    fn code(&self) -> Code<'_> {
        match self.0.kind {
            LibKind::Timeout => LIB_TIMEOUT,
            LibKind::Refused => LIB_REFUSED,
        }
    }

    fn recovery(&self) -> Recovery {
        match self.0.kind {
            LibKind::Timeout => Recovery::Retry,
            LibKind::Refused => Recovery::hint("ask the service owner for access"),
        }
    }
}

#[derive(Debug, crumbkit_derive::Failure)]
#[failure(crate = ::crumbkit)]
enum AppError {
    #[failure(forward)]
    Library(LibraryFailure),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Library(inner) = self;
        inner.fmt(f)
    }
}

impl std::error::Error for AppError {}

#[test]
fn a_forwarded_newtype_gives_the_answers_of_its_failure_impl_and_the_defaults() {
    let timeout = AppError::Library(
        LibError {
            kind: LibKind::Timeout,
        }
        .into(),
    );
    let refused = AppError::Library(
        LibError {
            kind: LibKind::Refused,
        }
        .into(),
    );
    assert_eq!(timeout.code(), LIB_TIMEOUT);
    assert_eq!(refused.code(), LIB_REFUSED);
    assert_eq!(timeout.recovery(), Recovery::Retry);
    assert_eq!(
        refused.recovery(),
        Recovery::hint("ask the service owner for access")
    );
    assert_eq!(timeout.alternatives(), Vec::new());
    assert_eq!(timeout.fields(), Vec::new());
    assert_eq!(timeout.causes(), Vec::<String>::new());
    assert_eq!(LibraryFailure::CODES, &[LIB_TIMEOUT, LIB_REFUSED]);
}
