//! The behaviour of `#[derive(Failure)]` on enums and structs, through
//! crumbkit's `Failure` trait, and of `#[derive(Row)]` on structs, through
//! crumbkit's `Row` trait and the output of `finish`.
//!
//! The tests name each derive by its full path, such as
//! `crumbkit_derive::Failure`, because a workspace build that turns on the
//! `derive` feature also brings each derive in with the trait of the same
//! name.

use std::fmt;

use crumbkit::{Call, Code, Failure, Field, FieldValue, Recovery};

const NOT_FOUND: Code<'static> = Code::new("shelf", "books", "not_found");
const REMOVED_FILE: Code<'static> = Code::new("shelf", "books", "removed_file");
const NO_STATE_DIRECTORY: Code<'static> = Code::new("shelf", "books", "no_state_directory");
const TIMED_OUT: Code<'static> = Code::new("shelf", "network", "timed_out");

/// The most ids a file may hold, which a hint names through an inline format
/// argument.
const MOST_IDS: usize = 100;

/// A failure that the enums below wrap.
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(
    crate = ::crumbkit,
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
#[failure(crate = ::crumbkit)]
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
    #[failure(code = NOT_FOUND, no_next_step = "the daemon owns {path}")]
    Owned { path: String },
    #[failure(code = NOT_FOUND, no_next_step = "the daemon owns the shelf")]
    Sealed,
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
#[failure(crate = ::crumbkit, forward = book)]
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
#[failure(crate = ::crumbkit, code = TIMED_OUT, recovery = Recovery::Retry)]
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
    assert_eq!(BooksError::NotFound { ids: vec![7] }.code(), NOT_FOUND);
    assert_eq!(BooksError::NoStateDirectory.code(), NO_STATE_DIRECTORY);
    assert_eq!(Timeout.code(), TIMED_OUT);
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
    assert_eq!(failure.code(), NOT_FOUND);
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
    assert_eq!(ReadingList::CODES, &[] as &[&Code<'static>]);
    assert_eq!(NoSuchBook::CODES, &[&NOT_FOUND]);
}

#[cfg(feature = "miette")]
mod diagnostic {
    use std::fmt;

    use crumbkit::{Code, Failure};
    use miette::{Diagnostic, LabeledSpan, SourceCode};

    const UNKNOWN_KEY: Code<'static> = Code::new("shelf", "config", "unknown_key");
    const NO_CONFIG: Code<'static> = Code::new("shelf", "config", "missing");

    /// A key that the settings file does not know, with a label on it.
    #[derive(Debug, crumbkit_derive::Failure)]
    #[failure(crate = ::crumbkit, code = UNKNOWN_KEY, hint = "use `color`", diagnostic)]
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
    #[failure(crate = ::crumbkit)]
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

#[test]
fn a_no_next_step_key_gives_the_reason_with_the_fields_that_it_names() {
    let failure = BooksError::Owned {
        path: "shelf.sock".to_owned(),
    };

    assert_eq!(
        failure.recovery(),
        Recovery::none("the daemon owns shelf.sock")
    );
}

#[test]
fn no_next_step_without_braces_is_the_reason_as_written() {
    assert_eq!(
        BooksError::Sealed.recovery(),
        Recovery::none("the daemon owns the shelf")
    );
}

/// A failure whose next step a person takes in a terminal.
#[derive(Debug, crumbkit_derive::Failure)]
#[failure(
    crate = ::crumbkit,
    code = TIMED_OUT,
    recovery = Recovery::Terminal(Call::new(&["books", "list"])),
)]
struct NeedsTerminal;

impl fmt::Display for NeedsTerminal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("needs a terminal")
    }
}

impl std::error::Error for NeedsTerminal {}

#[test]
fn a_recovery_attribute_takes_a_terminal_call() {
    assert_eq!(
        NeedsTerminal.recovery(),
        Recovery::Terminal(Call::new(&["books", "list"]))
    );
}

/// The behaviour of `#[derive(Row)]`.
mod rows {
    use clap::Parser;
    use crumbkit::{Document, Listing, Row, Sink, Tool, ViewFlags};
    use schemars::JsonSchema;
    use serde::Serialize;

    /// A book whose `Row` impl the derive writes.
    mod derived {
        use schemars::JsonSchema;
        use serde::Serialize;

        /// A book in the catalogue.
        #[derive(Serialize, JsonSchema, crumbkit_derive::Row)]
        #[row(crate = ::crumbkit)]
        pub(super) struct Book {
            /// The book's id.
            #[row(column, uncut)]
            pub(super) id: u32,
            /// The book's title.
            #[row(column)]
            pub(super) title: &'static str,
            /// The year the book came out.
            #[row(long)]
            pub(super) year: u16,
            /// What is wrong with the book's record, if anything.
            #[row(column, problem)]
            pub(super) warning: Option<&'static str>,
            /// The book's ISBN.
            pub(super) isbn: &'static str,
        }
    }

    /// The same book with a hand-written `Row` impl.
    mod written {
        use crumbkit::{Problem, Row};
        use schemars::JsonSchema;
        use serde::Serialize;

        /// A book in the catalogue.
        #[derive(Serialize, JsonSchema)]
        pub(super) struct Book {
            /// The book's id.
            pub(super) id: u32,
            /// The book's title.
            pub(super) title: &'static str,
            /// The year the book came out.
            pub(super) year: u16,
            /// What is wrong with the book's record, if anything.
            pub(super) warning: Option<&'static str>,
            /// The book's ISBN.
            pub(super) isbn: &'static str,
        }

        impl Row for Book {
            const COLUMNS: &'static [&'static str] = &["id", "title", "warning"];
            const LONG_COLUMNS: &'static [&'static str] = &["year"];
            const UNCUT_COLUMNS: &'static [&'static str] = &["id"];
            const PROBLEM: Option<Problem<Self>> = Some(Problem::field("warning"));
        }
    }

    const DERIVED: Document<Listing<derived::Book>> =
        Document::new("shelf", "books-list", 1, "The books.");
    const WRITTEN: Document<Listing<written::Book>> =
        Document::new("shelf", "books-list", 1, "The books.");

    /// A tool whose root takes the view flags.
    #[derive(Parser)]
    #[command(name = "shelf")]
    struct Cli {
        #[command(flatten)]
        view: ViewFlags,
    }

    /// Prints `rows` as one listing with the view flags `flags`, and returns
    /// the exit code and standard output, or `None` when the flags do not
    /// parse or the output is not UTF-8.
    fn print<R: Row>(
        document: Document<Listing<R>>,
        rows: Vec<R>,
        flags: &[&str],
    ) -> Option<(u8, String)> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let words = ["shelf"].iter().chain(flags);
        let parsed =
            crumbkit::parse_to::<Cli>(&mut Sink::new(&mut out, &mut err), &Tool::new(), words);
        let (_cli, run) = parsed.ok()?;
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = crumbkit::finish_to(
            &mut Sink::new(&mut out, &mut err),
            run,
            document,
            Ok::<_, crumbkit::Error>(Listing::from_all(rows, None)),
        );
        Some((code, String::from_utf8(out).ok()?))
    }

    #[test]
    fn a_derived_row_lists_its_marked_fields_in_field_order() {
        assert_eq!(derived::Book::COLUMNS, &["id", "title", "warning"]);
        assert_eq!(derived::Book::LONG_COLUMNS, &["year"]);
        assert_eq!(derived::Book::UNCUT_COLUMNS, &["id"]);
        let problem = derived::Book::PROBLEM.unwrap();
        assert_eq!(problem.get_field(), Some("warning"));
    }

    #[test]
    fn a_derived_row_reports_a_problem_through_its_problem_field() {
        let book = |warning| derived::Book {
            id: 1,
            title: "Frankenstein",
            year: 1818,
            warning,
            isbn: "",
        };
        let problem = derived::Book::PROBLEM.unwrap();
        assert!(problem.reports(&book(Some("no ISBN"))));
        assert!(!problem.reports(&book(None)));
    }

    #[test]
    fn a_derived_listing_prints_as_the_same_listing_of_a_hand_written_row() {
        let derived = || {
            vec![
                derived::Book {
                    id: 1,
                    title: "Frankenstein",
                    year: 1818,
                    warning: Some("no ISBN"),
                    isbn: "",
                },
                derived::Book {
                    id: 2,
                    title: "Moby-Dick",
                    year: 1851,
                    warning: None,
                    isbn: "978-0142437247",
                },
            ]
        };
        let written = || {
            derived()
                .into_iter()
                .map(|book| written::Book {
                    id: book.id,
                    title: book.title,
                    year: book.year,
                    warning: book.warning,
                    isbn: book.isbn,
                })
                .collect()
        };
        let views: [&[&str]; 5] = [
            &[],
            &["--long"],
            &["--json"],
            &["--json", "--full"],
            &["--fields", "id,isbn"],
        ];
        for flags in views {
            let from_derive = print(DERIVED, derived(), flags).unwrap();
            let by_hand = print(WRITTEN, written(), flags).unwrap();
            assert!(!from_derive.1.is_empty(), "{flags:?} printed nothing");
            assert_eq!(from_derive, by_hand, "{flags:?}");
        }
        assert_eq!(print(DERIVED, derived(), &[]).unwrap().0, 1);
    }

    /// Defines a row type `$name` whose type attribute is `#[serde(...)]`
    /// with the given keys, or none without keys. Two of its
    /// fields have no rename of their own, and two have one.
    macro_rules! renamed_row {
        ($name:ident $(, $($serde:tt)+)?) => {
            #[derive(Serialize, JsonSchema, crumbkit_derive::Row)]
            $(#[serde($($serde)+)])?
            struct $name {
                #[row(column)]
                id: u32,
                #[row(column)]
                first_page_count: u32,
                #[row(long)]
                #[serde(rename = "Pages")]
                page_total: u32,
                #[row(long)]
                #[serde(rename(serialize = "Shelf", deserialize = "shelf"))]
                shelf_name: &'static str,
            }

            impl $name {
                /// Returns one row of this type.
                fn sample() -> Self {
                    Self {
                        id: 1,
                        first_page_count: 2,
                        page_total: 3,
                        shelf_name: "fiction",
                    }
                }
            }
        };
    }

    renamed_row!(Plain);
    renamed_row!(Lower, rename_all = "lowercase");
    renamed_row!(Upper, rename_all = "UPPERCASE");
    renamed_row!(Pascal, rename_all = "PascalCase");
    renamed_row!(Camel, rename_all = "camelCase");
    renamed_row!(Snake, rename_all = "snake_case");
    renamed_row!(ScreamingSnake, rename_all = "SCREAMING_SNAKE_CASE");
    renamed_row!(Kebab, rename_all = "kebab-case");
    renamed_row!(ScreamingKebab, rename_all = "SCREAMING-KEBAB-CASE");
    renamed_row!(KebabForSerialize, rename_all(serialize = "kebab-case"));

    /// Checks that `R`'s columns are `columns` and its long columns are the
    /// two renamed fields, and that every one of them is a key of `row`'s
    /// JSON, which holds no other key.
    fn assert_names_are_keys<R: Row>(row: &R, columns: &[&str]) {
        assert_eq!(R::COLUMNS, columns);
        assert_eq!(R::LONG_COLUMNS, &["Pages", "Shelf"]);
        let json = serde_json::to_value(row).ok();
        let mut keys: Option<Vec<&str>> = json
            .as_ref()
            .and_then(serde_json::Value::as_object)
            .map(|object| object.keys().map(String::as_str).collect());
        let mut names: Vec<&str> = R::COLUMNS.iter().chain(R::LONG_COLUMNS).copied().collect();
        if let Some(keys) = keys.as_mut() {
            keys.sort_unstable();
        }
        names.sort_unstable();
        assert_eq!(Some(names), keys);
    }

    #[test]
    fn every_derived_name_is_a_key_of_the_rows_json() {
        assert_names_are_keys(&Plain::sample(), &["id", "first_page_count"]);
        assert_names_are_keys(&Lower::sample(), &["id", "first_page_count"]);
        assert_names_are_keys(&Upper::sample(), &["ID", "FIRST_PAGE_COUNT"]);
        assert_names_are_keys(&Pascal::sample(), &["Id", "FirstPageCount"]);
        assert_names_are_keys(&Camel::sample(), &["id", "firstPageCount"]);
        assert_names_are_keys(&Snake::sample(), &["id", "first_page_count"]);
        assert_names_are_keys(&ScreamingSnake::sample(), &["ID", "FIRST_PAGE_COUNT"]);
        assert_names_are_keys(&Kebab::sample(), &["id", "first-page-count"]);
        assert_names_are_keys(&ScreamingKebab::sample(), &["ID", "FIRST-PAGE-COUNT"]);
        assert_names_are_keys(&KebabForSerialize::sample(), &["id", "first-page-count"]);
    }

    /// A row that holds a value of any printable type.
    #[derive(Serialize, JsonSchema, crumbkit_derive::Row)]
    struct Labelled<T: Serialize + JsonSchema> {
        /// What the value is.
        #[row(column)]
        label: &'static str,
        /// The value.
        #[row(column)]
        value: T,
    }

    const LABELLED: Document<Listing<Labelled<u32>>> =
        Document::new("shelf", "counts", 1, "The counts.");

    #[test]
    fn a_generic_row_type_derives_row() {
        let rows = vec![Labelled {
            label: "books",
            value: 3_u32,
        }];
        assert_eq!(Labelled::<u32>::COLUMNS, &["label", "value"]);
        assert_eq!(
            print(LABELLED, rows, &[]),
            Some((0, "LABEL  VALUE\nbooks  3\n".to_owned()))
        );
    }
}
