//! The behaviour of `#[derive(Row)]` on structs, through crumbkit's `Row`
//! trait and the output of `finish`.
//!
//! The tests name the derive by its full path, `crumbkit_derive::Row`,
//! because a workspace build that turns on the `derive` feature also brings
//! the derive in with the trait of the same name.

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
        pub(super) damage: Option<&'static str>,
        /// How worn the book is, if at all.
        #[row(column, warning)]
        pub(super) wear: Option<&'static str>,
        /// The book's ISBN.
        pub(super) isbn: &'static str,
    }
}

/// The same book with a hand-written `Row` impl.
mod written {
    use crumbkit::{Problem, Row, Warning};
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
        pub(super) damage: Option<&'static str>,
        /// How worn the book is, if at all.
        pub(super) wear: Option<&'static str>,
        /// The book's ISBN.
        pub(super) isbn: &'static str,
    }

    impl Row for Book {
        const COLUMNS: &'static [&'static str] = &["id", "title", "damage", "wear"];
        const LONG_COLUMNS: &'static [&'static str] = &["year"];
        const UNCUT_COLUMNS: &'static [&'static str] = &["id"];
        const PROBLEM: Option<Problem<Self>> = Some(Problem::field("damage"));
        const WARNING: Option<Warning<Self>> = Some(Warning::field("wear"));
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
    let parsed = crumbkit::parse_to::<Cli>(&mut Sink::new(&mut out, &mut err), &Tool::new(), words);
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
    assert_eq!(derived::Book::COLUMNS, &["id", "title", "damage", "wear"]);
    assert_eq!(derived::Book::LONG_COLUMNS, &["year"]);
    assert_eq!(derived::Book::UNCUT_COLUMNS, &["id"]);
    let problem = derived::Book::PROBLEM.unwrap();
    assert_eq!(problem.get_field(), Some("damage"));
    let warning = derived::Book::WARNING.unwrap();
    assert_eq!(warning.get_field(), Some("wear"));
}

#[test]
fn a_derived_row_reports_a_problem_through_its_problem_field() {
    let book = |damage| derived::Book {
        id: 1,
        title: "Frankenstein",
        year: 1818,
        damage,
        wear: None,
        isbn: "",
    };
    let problem = derived::Book::PROBLEM.unwrap();
    assert!(problem.reports(&book(Some("no ISBN"))));
    assert!(!problem.reports(&book(None)));
}

#[test]
fn a_derived_row_reports_a_warning_through_its_warning_field() {
    let book = |wear| derived::Book {
        id: 1,
        title: "Frankenstein",
        year: 1818,
        damage: None,
        wear,
        isbn: "",
    };
    let warning = derived::Book::WARNING.unwrap();
    assert!(warning.reports(&book(Some("frayed"))));
    assert!(!warning.reports(&book(None)));
}

#[test]
fn a_derived_listing_prints_as_the_same_listing_of_a_hand_written_row() {
    let derived = || {
        vec![
            derived::Book {
                id: 1,
                title: "Frankenstein",
                year: 1818,
                damage: Some("no ISBN"),
                wear: Some("frayed"),
                isbn: "",
            },
            derived::Book {
                id: 2,
                title: "Moby-Dick",
                year: 1851,
                damage: None,
                wear: None,
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
                damage: book.damage,
                wear: book.wear,
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

/// A service and the parts it runs on, with child lines, `whole` and
/// `problem_by`. The derived type's impl is written by the derive, and the
/// written type's impl by hand.
mod service {
    use crumbkit::{Children, Problem, Row};
    use schemars::JsonSchema;
    use serde::Serialize;

    /// A service and the parts it runs on.
    #[derive(Serialize, JsonSchema, crumbkit_derive::Row)]
    #[row(crate = ::crumbkit, whole)]
    pub(super) struct Derived {
        /// The service's name.
        #[row(column)]
        pub(super) name: &'static str,
        /// How many parts are down.
        #[row(column, problem_by = |row: &Self| row.down > 0)]
        pub(super) down: u32,
        /// The region the service runs in.
        #[row(long)]
        pub(super) region: &'static str,
        /// The parts that the service runs on.
        #[row(children(
            indent = 4,
            spaced,
            help = "one line for each part.",
            lines = Self::part_lines,
            long_lines = |row: &Self| vec![format!("long {}", row.name)],
        ))]
        pub(super) parts: Vec<&'static str>,
    }

    impl Derived {
        /// Returns one line for each part.
        fn part_lines(&self) -> Vec<String> {
            self.parts
                .iter()
                .map(|part| format!("part {part}"))
                .collect()
        }
    }

    /// A service and the parts it runs on.
    #[derive(Serialize, JsonSchema)]
    pub(super) struct Written {
        /// The service's name.
        pub(super) name: &'static str,
        /// How many parts are down.
        pub(super) down: u32,
        /// The region the service runs in.
        pub(super) region: &'static str,
        /// The parts that the service runs on.
        pub(super) parts: Vec<&'static str>,
    }

    impl Written {
        /// Returns one line for each part.
        fn part_lines(&self) -> Vec<String> {
            self.parts
                .iter()
                .map(|part| format!("part {part}"))
                .collect()
        }
    }

    impl Row for Written {
        const COLUMNS: &'static [&'static str] = &["name", "down"];
        const LONG_COLUMNS: &'static [&'static str] = &["region"];
        const PROBLEM: Option<Problem<Self>> = Some(Problem::by("down", |row| row.down > 0));
        const CHILDREN: Option<Children<Self>> = Some(
            Children::field("parts")
                .indent(4)
                .spaced()
                .help("one line for each part.")
                .lines(Self::part_lines)
                .long_lines(|row| vec![format!("long {}", row.name)]),
        );
        const WHOLE: bool = true;
    }

    /// Returns two services as derived rows, the second with a part down.
    pub(super) fn derived() -> Vec<Derived> {
        vec![
            Derived {
                name: "catalogue",
                down: 0,
                region: "north",
                parts: vec!["database", "cache"],
            },
            Derived {
                name: "loans",
                down: 1,
                region: "south",
                parts: vec!["queue"],
            },
        ]
    }

    /// Returns the same two services as written rows.
    pub(super) fn written() -> Vec<Written> {
        derived()
            .into_iter()
            .map(|row| Written {
                name: row.name,
                down: row.down,
                region: row.region,
                parts: row.parts,
            })
            .collect()
    }
}

const SERVICES_DERIVED: Document<Listing<service::Derived>> =
    Document::new("shelf", "services", 1, "The services.");
const SERVICES_WRITTEN: Document<Listing<service::Written>> =
    Document::new("shelf", "services", 1, "The services.");

#[test]
fn a_derived_row_with_children_prints_the_same_listing_as_a_hand_written_row() {
    let views: [&[&str]; 3] = [&[], &["--long"], &["--fields", "name"]];
    for flags in views {
        let from_derive = print(SERVICES_DERIVED, service::derived(), flags).unwrap();
        let by_hand = print(SERVICES_WRITTEN, service::written(), flags).unwrap();
        assert!(!from_derive.1.is_empty(), "{flags:?} printed nothing");
        assert_eq!(from_derive, by_hand, "{flags:?}");
    }
    let text = print(SERVICES_DERIVED, service::derived(), &[]).unwrap().1;
    assert!(
        text.contains(
            "    part database
    part cache

"
        ),
        "{text}"
    );
}

#[test]
fn a_derived_whole_row_prints_the_same_json_as_a_hand_written_row() {
    let from_derive = print(SERVICES_DERIVED, service::derived(), &["--json"]).unwrap();
    let by_hand = print(SERVICES_WRITTEN, service::written(), &["--json"]).unwrap();
    assert!(from_derive.1.contains("\"parts\""), "{}", from_derive.1);
    assert_eq!(from_derive, by_hand);
}

#[test]
fn a_derived_problem_by_row_exits_with_1_as_a_hand_written_row_does() {
    let from_derive = print(SERVICES_DERIVED, service::derived(), &[]).unwrap();
    let by_hand = print(SERVICES_WRITTEN, service::written(), &[]).unwrap();
    assert_eq!(from_derive.0, 1);
    assert_eq!(from_derive.0, by_hand.0);
    let healthy = print(
        SERVICES_DERIVED,
        service::derived().into_iter().take(1).collect(),
        &[],
    );
    assert_eq!(healthy.map(|(code, _)| code), Some(0));
}

/// A file whose size and line count come from a flattened field.
mod flattened {
    use schemars::JsonSchema;
    use serde::Serialize;

    /// The measures of a file.
    #[derive(Serialize, JsonSchema, crumbkit_derive::Row)]
    #[row(crate = ::crumbkit)]
    pub(super) struct Stats {
        /// The file's size in bytes.
        #[row(column)]
        pub(super) size: u64,
        /// The file's line count.
        #[row(long)]
        pub(super) lines: u64,
    }

    /// A file and its measures.
    #[derive(Serialize, JsonSchema, crumbkit_derive::Row)]
    #[row(crate = ::crumbkit)]
    pub(super) struct File {
        /// The file's path.
        #[row(column, uncut)]
        pub(super) path: &'static str,
        /// The file's measures.
        #[serde(flatten)]
        #[row(flatten)]
        pub(super) stats: Stats,
        /// The file's owner.
        #[row(long)]
        pub(super) owner: &'static str,
        /// The file's mode.
        #[row(column)]
        pub(super) mode: &'static str,
    }
}

const FILES: Document<Listing<flattened::File>> = Document::new("shelf", "files", 1, "The files.");

#[test]
fn flattened_columns_print_in_field_order_and_their_long_columns_follow_long() {
    assert_eq!(flattened::File::COLUMNS, &["path", "size", "mode"]);
    assert_eq!(flattened::File::LONG_COLUMNS, &["lines", "owner"]);
    assert_eq!(flattened::File::UNCUT_COLUMNS, &["path"]);
    let files = || {
        vec![flattened::File {
            path: "a.txt",
            stats: flattened::Stats { size: 12, lines: 3 },
            owner: "root",
            mode: "644",
        }]
    };
    assert_eq!(
        print(FILES, files(), &[]),
        Some((0, "PATH   SIZE  MODE\na.txt  12    644\n".to_owned()))
    );
    assert_eq!(
        print(FILES, files(), &["--long"]),
        Some((
            0,
            "PATH   SIZE  MODE  LINES  OWNER\na.txt  12    644   3      root\n".to_owned()
        ))
    );
}

#[test]
fn the_columns_helper_concatenates_lists_in_order() {
    const JOINED: [&str; 3] = crumbkit::__row_columns(&[&["a"], &["b", "c"], &[]]);
    assert_eq!(JOINED, ["a", "b", "c"]);
}

/// A foreign row type, such as another crate's report, and the newtype that
/// wraps it, whose `Row` impl the derive writes through a local mirror.
mod remote {
    use schemars::JsonSchema;
    use serde::Serialize;

    /// A run report, as another crate defines it.
    #[derive(Serialize, JsonSchema)]
    #[serde(rename_all = "camelCase")]
    pub(super) struct RunReport {
        /// The run's name.
        pub(super) run_name: &'static str,
        /// The items the run handled.
        pub(super) item_count: u32,
        /// Whether the run failed.
        pub(super) failed: bool,
    }

    /// A newtype over any printable value, which prints as the value.
    #[derive(Serialize, JsonSchema)]
    #[serde(transparent)]
    pub(super) struct Printed<T>(pub(super) T);

    /// The same newtype, whose impl is written by hand.
    #[derive(Serialize, JsonSchema)]
    #[serde(transparent)]
    pub(super) struct PrintedByHand<T>(pub(super) T);

    /// Lists the fields of `Printed<RunReport>` that its rows print. The
    /// mirror is never built and its fields are never read.
    #[derive(crumbkit_derive::Row)]
    #[row(crate = ::crumbkit, remote = Printed<RunReport>)]
    #[serde(rename_all = "camelCase")]
    struct PrintedRunReport {
        #[row(column, uncut)]
        run_name: String,
        #[row(column, problem_by = |row: &Self| row.0.failed)]
        failed: bool,
        #[row(long)]
        item_count: u32,
    }

    impl crumbkit::Row for PrintedByHand<RunReport> {
        const COLUMNS: &'static [&'static str] = &["runName", "failed"];
        const LONG_COLUMNS: &'static [&'static str] = &["itemCount"];
        const UNCUT_COLUMNS: &'static [&'static str] = &["runName"];
        const PROBLEM: Option<crumbkit::Problem<Self>> =
            Some(crumbkit::Problem::by("failed", |row| row.0.failed));
    }

    /// Returns two reports as the derived newtype.
    pub(super) fn derived() -> Vec<Printed<RunReport>> {
        vec![
            Printed(RunReport {
                run_name: "nightly",
                item_count: 4,
                failed: false,
            }),
            Printed(RunReport {
                run_name: "weekly",
                item_count: 9,
                failed: true,
            }),
        ]
    }

    /// Returns the same reports as the hand-written newtype.
    pub(super) fn written() -> Vec<PrintedByHand<RunReport>> {
        derived()
            .into_iter()
            .map(|printed| PrintedByHand(printed.0))
            .collect()
    }
}

const REMOTE_DERIVED: Document<Listing<remote::Printed<remote::RunReport>>> =
    Document::new("shelf", "runs", 1, "The runs.");
const REMOTE_WRITTEN: Document<Listing<remote::PrintedByHand<remote::RunReport>>> =
    Document::new("shelf", "runs", 1, "The runs.");

#[test]
fn a_remote_mirror_gives_a_newtype_the_same_output_as_a_hand_written_impl() {
    let views: [&[&str]; 3] = [&[], &["--long"], &["--json"]];
    for flags in views {
        let from_derive = print(REMOTE_DERIVED, remote::derived(), flags).unwrap();
        let by_hand = print(REMOTE_WRITTEN, remote::written(), flags).unwrap();
        assert!(!from_derive.1.is_empty(), "{flags:?} printed nothing");
        assert_eq!(from_derive, by_hand, "{flags:?}");
    }
    let text = print(REMOTE_DERIVED, remote::derived(), &["--long"]).unwrap();
    assert_eq!(text.0, 1);
    assert!(text.1.contains("RUNNAME"), "{}", text.1);
}
