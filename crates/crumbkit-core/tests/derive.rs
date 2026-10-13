//! The behaviour of `#[derive(Failure)]` on enums and structs, through
//! crumbkit-core's `Failure` trait.
//!
//! The tests name the derive by its full path, `crumbkit_derive::Failure`,
//! because a workspace build that turns on the `derive` feature also brings
//! the derive in with the `Failure` trait.

use std::fmt;

use crumbkit_core::{Call, Code, Failure, Field, FieldValue, Recovery};

const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
const REMOVED_FILE: Code = Code::new("shelf", "books", "removed_file");
const NO_STATE_DIRECTORY: Code = Code::new("shelf", "books", "no_state_directory");
const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");

/// The most ids a file may hold, which a hint names through an inline format
/// argument.
const MOST_IDS: usize = 100;

/// A failure that the enums below wrap.
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(
    crate = ::crumbkit_core,
    code = NOT_FOUND,
    recovery = Recovery::Run(Call::new(&["books", "list"])),
    alternative = Recovery::Retry,
    field(book = *id),
    url = "https://example.org/books",
)]
struct NoSuchBook {
    id: u32,
}

impl fmt::Display for NoSuchBook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the catalogue has no book {}", self.id)
    }
}

impl std::error::Error for NoSuchBook {}

#[derive(Debug, crumbkit_derive::Failure)]
#[failure(crate = ::crumbkit_core)]
enum BooksError {
    #[failure(
        code = NOT_FOUND,
        hint = "list the books",
        field(books = FieldValue::list(ids.iter().copied())),
        field(count = ids.len()),
    )]
    NotFound { ids: Vec<u32> },
    #[failure(
        code = REMOVED_FILE,
        hint = "make {name} hold at most {MOST_IDS} ids",
        alternative = Recovery::Run(Call::new(&["books", "show"])),
        alternative = Recovery::Hint(format!("delete {name}")),
        url = "https://example.org/removed",
    )]
    RemovedFile { name: String },
    #[failure(code = NO_STATE_DIRECTORY, recovery = Recovery::Hint(format!("set HOME, {}", self.reason())))]
    NoStateDirectory,
    #[failure(code = REMOVED_FILE, hint = "check the file")]
    Unreadable(String),
    #[failure(forward)]
    Book(NoSuchBook),
}

impl BooksError {
    /// Returns a reason that a recovery expression reads through `self`.
    fn reason(&self) -> &'static str {
        match self {
            Self::NoStateDirectory => "because shelf needs a state directory",
            _ => "",
        }
    }
}

impl fmt::Display for BooksError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a books failure")
    }
}

impl std::error::Error for BooksError {}

/// A struct that wraps a failure in a named field beside other context.
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(crate = ::crumbkit_core, forward = book)]
struct ReadingList {
    name: String,
    book: NoSuchBook,
}

impl fmt::Display for ReadingList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot read the reading list {}", self.name)
    }
}

impl std::error::Error for ReadingList {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.book)
    }
}

/// A struct with no field.
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(crate = ::crumbkit_core, code = TIMED_OUT, recovery = Recovery::Retry)]
struct Timeout;

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("timed out")
    }
}

impl std::error::Error for Timeout {}

fn names<'a>(fields: &'a [Field<'a>]) -> Vec<&'a str> {
    fields.iter().map(Field::name).collect()
}

#[test]
fn a_variant_returns_the_code_that_its_attribute_names() {
    assert_eq!(BooksError::NotFound { ids: vec![7] }.code(), &NOT_FOUND);
    assert_eq!(BooksError::NoStateDirectory.code(), &NO_STATE_DIRECTORY);
    assert_eq!(Timeout.code(), &TIMED_OUT);
}

#[test]
fn a_hint_fills_in_the_variant_fields_and_constants_that_it_names_in_braces() {
    let failure = BooksError::RemovedFile {
        name: "removed.txt".to_owned(),
    };
    assert_eq!(
        failure.recovery(),
        Recovery::Hint("make removed.txt hold at most 100 ids".to_owned())
    );
}

#[test]
fn a_hint_without_braces_is_returned_as_written() {
    let failure = BooksError::NotFound { ids: vec![7] };
    assert_eq!(
        failure.recovery(),
        Recovery::Hint("list the books".to_owned())
    );
}

#[test]
fn a_recovery_expression_can_call_a_method_of_the_failure() {
    assert_eq!(
        BooksError::NoStateDirectory.recovery(),
        Recovery::Hint("set HOME, because shelf needs a state directory".to_owned())
    );
}

#[test]
fn alternatives_are_returned_in_the_order_they_are_written() {
    let failure = BooksError::RemovedFile {
        name: "removed.txt".to_owned(),
    };
    assert_eq!(
        failure.alternatives(),
        vec![
            Recovery::Run(Call::new(&["books", "show"])),
            Recovery::Hint("delete removed.txt".to_owned()),
        ]
    );
}

#[test]
fn fields_are_returned_in_the_order_they_are_written() {
    let failure = BooksError::NotFound { ids: vec![7, 9] };
    let fields = failure.fields();
    assert_eq!(names(&fields), ["books", "count"]);
    assert_eq!(fields[0].value(), &FieldValue::list([7_u32, 9_u32]));
    assert_eq!(fields[1].value(), &FieldValue::from(2_usize));
}

#[test]
fn a_variant_without_alternatives_fields_or_a_url_returns_none_of_them() {
    let failure = BooksError::Unreadable("ids.txt".to_owned());
    assert_eq!(failure.alternatives(), Vec::new());
    assert_eq!(failure.fields(), Vec::new());
    assert_eq!(failure.url(), None);
}

#[test]
fn a_url_is_returned_for_its_variant_only() {
    let removed = BooksError::RemovedFile {
        name: "removed.txt".to_owned(),
    };
    assert_eq!(removed.url(), Some("https://example.org/removed"));
    assert_eq!(BooksError::NoStateDirectory.url(), None);
}

#[test]
fn a_forwarding_variant_returns_every_answer_of_the_failure_it_wraps() {
    let failure = BooksError::Book(NoSuchBook { id: 7 });
    let inner = NoSuchBook { id: 7 };
    assert_eq!(failure.code(), inner.code());
    assert_eq!(failure.recovery(), inner.recovery());
    assert_eq!(failure.alternatives(), inner.alternatives());
    assert_eq!(failure.fields(), inner.fields());
    assert_eq!(failure.url(), inner.url());
}

#[test]
fn a_forwarding_struct_returns_every_answer_of_the_named_field() {
    let failure = ReadingList {
        name: "summer".to_owned(),
        book: NoSuchBook { id: 7 },
    };
    assert_eq!(failure.code(), &NOT_FOUND);
    assert_eq!(
        failure.recovery(),
        Recovery::Run(Call::new(&["books", "list"]))
    );
    assert_eq!(failure.alternatives(), vec![Recovery::Retry]);
    assert_eq!(names(&failure.fields()), ["book"]);
    assert_eq!(failure.url(), Some("https://example.org/books"));
}

#[test]
fn the_codes_list_holds_each_code_once_in_variant_order() {
    assert_eq!(
        BooksError::CODES,
        &[&NOT_FOUND, &REMOVED_FILE, &NO_STATE_DIRECTORY]
    );
    assert_eq!(Timeout::CODES, &[&TIMED_OUT]);
}

#[test]
fn the_codes_list_leaves_out_forwarding_variants() {
    assert_eq!(ReadingList::CODES, &[] as &[&Code]);
    assert_eq!(NoSuchBook::CODES, &[&NOT_FOUND]);
}

#[cfg(feature = "miette")]
mod diagnostic {
    use std::fmt;

    use crumbkit_core::{Code, Failure};
    use miette::{Diagnostic, LabeledSpan, SourceCode};

    const UNKNOWN_KEY: Code = Code::new("shelf", "config", "unknown_key");
    const NO_CONFIG: Code = Code::new("shelf", "config", "missing");

    /// A key that the settings file does not know, with a label on it.
    #[derive(Debug, crumbkit_derive::Failure)]
    #[failure(crate = ::crumbkit_core, code = UNKNOWN_KEY, hint = "use `color`", diagnostic)]
    struct UnknownKey {
        source: String,
    }

    impl fmt::Display for UnknownKey {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("unknown key")
        }
    }

    impl std::error::Error for UnknownKey {}

    impl Diagnostic for UnknownKey {
        fn source_code(&self) -> Option<&dyn SourceCode> {
            Some(&self.source)
        }

        fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
            let label = LabeledSpan::new(Some("this key".to_owned()), 0, 6);
            Some(Box::new(std::iter::once(label)))
        }
    }

    #[derive(Debug, crumbkit_derive::Failure)]
    #[failure(crate = ::crumbkit_core)]
    enum ConfigError {
        #[failure(forward, diagnostic)]
        Key(UnknownKey),
        #[failure(forward)]
        Quiet(UnknownKey),
        #[failure(code = NO_CONFIG, hint = "create the file")]
        Missing,
    }

    impl fmt::Display for ConfigError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a settings failure")
        }
    }

    impl std::error::Error for ConfigError {}

    fn unknown() -> UnknownKey {
        UnknownKey {
            source: "colour = red".to_owned(),
        }
    }

    #[test]
    fn a_variant_marked_diagnostic_returns_itself_as_its_diagnostic() {
        let failure = unknown();
        let labels: Vec<LabeledSpan> = failure
            .diagnostic()
            .and_then(Diagnostic::labels)
            .map(Iterator::collect)
            .unwrap_or_default();
        assert_eq!(labels.len(), 1);
    }

    #[test]
    fn a_forward_marked_diagnostic_returns_the_diagnostic_of_the_failure_it_wraps() {
        assert!(ConfigError::Key(unknown()).diagnostic().is_some());
        assert!(ConfigError::Quiet(unknown()).diagnostic().is_none());
        assert!(ConfigError::Missing.diagnostic().is_none());
    }
}
