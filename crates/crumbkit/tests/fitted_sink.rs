//! A tool outside crumbkit that prints through its own sink and gives the
//! sink the width of its terminal, so `finish_to` fits a text table to that
//! width as `finish` fits it to the terminal (ADR-0121, ADR-0134).

use std::ops::ControlFlow;
use std::process::ExitCode;

use clap::{CommandFactory as _, Parser, Subcommand};
use crumbkit::help::Guide;
use crumbkit::{Document, Error, Listing, Output, Row, Sink, Stream, ViewFlags};
use schemars::JsonSchema;
use serde::Serialize;

/// A tool with a `list` command and a `watch` command.
#[derive(Parser)]
#[command(name = "library")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    /// Lists the books.
    List,
    /// Prints each book as it arrives.
    Watch,
}

/// A book whose `isbn` is an identifier that a person copies, so a fitted
/// table never cuts it.
#[derive(Serialize, JsonSchema)]
struct Book {
    /// The book's ISBN.
    isbn: &'static str,
    /// The book's title.
    title: String,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["isbn", "title"];
    const UNCUT_COLUMNS: &'static [&'static str] = &["isbn"];
}

const BOOKS: Document<Vec<Book>> = Document::new("library", "books", 1, "The books.");

const BOOK_LISTING: Document<Listing<Book>> =
    Document::new("library", "book-listing", 1, "One page of the books.");

const BOOK_STREAM: Document<Stream<Book>> =
    Document::new("library", "book-stream", 1, "Each book as it arrives.");

/// Returns two books with long titles.
fn books() -> Vec<Book> {
    vec![
        Book {
            isbn: "978-0141439518",
            title: "Pride and Prejudice".to_owned(),
        },
        Book {
            isbn: "978-0486424538",
            title: "The Adventures of Sherlock Holmes".to_owned(),
        },
    ]
}

/// Parses `library` followed by `args` with `guide`, and returns the
/// output, or `None` when the line does not parse.
fn parse(guide: &Guide, args: &[&str]) -> Option<Output> {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let words = ["library"].iter().chain(args);
    let parsed = crumbkit::parse_to::<Tool, _, _, _, _>(
        &mut Sink::new(&mut out, &mut err),
        Tool::command(),
        guide,
        words,
    );
    parsed.ok().map(|(_tool, output)| output)
}

/// What a run printed on each stream.
struct Printed {
    out: String,
    err: String,
}

/// Finishes `value` from `library list` with `flags` and `guide` into the
/// tool's own sink, which `width` gives its width, or returns `None` when
/// the line does not parse or the output is not UTF-8.
fn list<T: crumbkit::Rows>(
    guide: &Guide,
    flags: &[&str],
    document: Document<T>,
    value: T,
    width: Option<usize>,
) -> Option<Printed> {
    let output = parse(guide, &[&["list"], flags].concat())?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    let mut sink = Sink::new(&mut out, &mut err).fitted(width);
    let code = crumbkit::finish_to(&mut sink, output, document, Ok::<_, Error>(value));
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    Some(Printed {
        out: String::from_utf8(out).ok()?,
        err: String::from_utf8(err).ok()?,
    })
}

/// Finishes the two books from `library list` with `flags` into the tool's
/// own sink, which `width` gives its width, or returns `None` when the line
/// does not parse.
fn list_books(flags: &[&str], width: Option<usize>) -> Option<Printed> {
    list(&Guide::new(), flags, BOOKS, books(), width)
}

#[test]
fn finish_to_with_a_fitted_sink_narrows_a_text_table_wider_than_the_width() {
    let printed = list_books(&[], Some(31)).unwrap();

    let expected = "\
ISBN            TITLE
978-0141439518  Pride and Prej…
978-0486424538  The Adventures…
";
    assert_eq!(printed.out, expected);
    assert_eq!(printed.err, "");
}

#[test]
fn finish_to_with_a_fitted_sink_keeps_an_uncut_column_whole() {
    let printed = list_books(&["--fields", "title,isbn"], Some(31)).unwrap();

    let expected = "\
TITLE            ISBN
Pride and Prej…  978-0141439518
The Adventures…  978-0486424538
";
    assert_eq!(printed.out, expected);
}

#[test]
fn finish_to_with_an_unfitted_sink_prints_the_table_at_full_width() {
    let printed = list_books(&[], None).unwrap();

    let expected = "\
ISBN            TITLE
978-0141439518  Pride and Prejudice
978-0486424538  The Adventures of Sherlock Holmes
";
    assert_eq!(printed.out, expected);
}

#[test]
fn a_width_narrower_than_the_headers_and_uncut_columns_leaves_the_table_unfitted() {
    let unfitted = list_books(&[], None).unwrap();

    for width in [0, 1, 20] {
        assert_eq!(
            list_books(&[], Some(width)).unwrap().out,
            unfitted.out,
            "{width}"
        );
    }
}

#[test]
fn json_through_a_fitted_sink_is_the_same_as_through_an_unfitted_sink() {
    let fitted = list_books(&["--json"], Some(20)).unwrap();
    let unfitted = list_books(&["--json"], None).unwrap();

    assert_eq!(fitted.out, unfitted.out);
    assert!(
        fitted.out.contains("The Adventures of Sherlock Holmes"),
        "{}",
        fitted.out
    );
}

#[test]
fn the_output_budget_counts_the_unfitted_table_through_a_fitted_sink() {
    let guide = Guide::new().output_budget(1024, "the tests print little");
    let listing = || {
        let rows: Vec<Book> = (0..30)
            .map(|number| Book {
                isbn: "978-0141439518",
                title: format!("{number:02} {}", "a".repeat(60)),
            })
            .collect();
        Listing::new(rows, 30).described("books")
    };

    let fitted = list(&guide, &[], BOOK_LISTING, listing(), Some(40)).unwrap();
    let unfitted = list(&guide, &[], BOOK_LISTING, listing(), None).unwrap();

    assert_ne!(fitted.out, unfitted.out, "the table was not fitted");
    assert_eq!(fitted.out.lines().count(), unfitted.out.lines().count());
    assert_eq!(fitted.err, unfitted.err);
    assert!(
        unfitted.err.starts_with("Showing "),
        "the budget cut no row: {}",
        unfitted.err
    );
}

#[test]
fn a_stream_through_a_fitted_sink_prints_its_table_unfitted() {
    let watch = |width: Option<usize>| {
        let output = parse(&Guide::new(), &["watch"]).unwrap();
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut sink = Sink::new(&mut out, &mut err).fitted(width);
        let mut streaming = crumbkit::stream_to(&mut sink, output, BOOK_STREAM).unwrap();
        for book in books() {
            assert_eq!(streaming.item(&book), ControlFlow::Continue(()));
        }
        assert_eq!(streaming.end(Ok::<(), Error>(())), ExitCode::SUCCESS);
        String::from_utf8(out).unwrap()
    };

    let fitted = watch(Some(20));

    assert_eq!(fitted, watch(None));
    assert!(
        fitted.contains("The Adventures of Sherlock Holmes"),
        "{fitted}"
    );
}

/// A book whose catalogue reference, an identifier that a reader copies
/// whole, prints as its one child line at column 1 (ADR-0135).
#[derive(Serialize, JsonSchema)]
struct Catalogued {
    /// The book's title.
    title: String,
    /// The book's shelf.
    shelf: &'static str,
    /// The identifier that another command takes.
    reference: String,
}

impl Row for Catalogued {
    const COLUMNS: &'static [&'static str] = &["title", "shelf"];
    const CHILDREN: Option<&'static str> = Some("reference");
    const CHILD_INDENT: u8 = 0;

    fn child_lines(&self) -> Vec<String> {
        vec![self.reference.clone()]
    }
}

#[test]
fn a_fitted_table_fits_the_width_while_a_child_line_wider_than_the_width_prints_whole() {
    const CATALOGUE: Document<Vec<Catalogued>> =
        Document::new("library", "catalogue", 1, "The catalogue.");
    let reference = format!("cat:{}", "0123456789".repeat(4));
    let rows = vec![Catalogued {
        title: "The Adventures of Sherlock Holmes".to_owned(),
        shelf: "crime",
        reference: reference.clone(),
    }];

    let printed = list(&Guide::new(), &[], CATALOGUE, rows, Some(30)).unwrap();

    let expected =
        format!("TITLE                    SHELF\nThe Adventures of Sher…  crime\n{reference}\n");
    assert_eq!(printed.out, expected);
    let lines: Vec<&str> = printed.out.lines().collect();
    for line in &lines[..2] {
        assert!(line.chars().count() <= 30, "{line}");
    }
    assert_eq!(lines[2].len(), 44);
}
