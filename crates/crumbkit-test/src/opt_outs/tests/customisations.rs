//! Tests for the customisations section of the opt-out report, which lists
//! the tool's declarations without a reason or a cost.

use clap::Command;
use crumbkit::codes::CLI_USAGE;
use crumbkit::help::{Exit, Guide, Leaf, RefusalContext, Rename};
use crumbkit::schemas::SCHEMAS_LIST;
use crumbkit::{Call, Code, Document, Prose};
use serde_json::{Value, json};
use snapbox::assert::Action;

use super::{read, run_guided};
use crate::opt_outs::customisations::customisations;
use crate::temp::TempDir;

/// Returns the call that a refused `books show` command line offers instead.
fn version_call(_: &RefusalContext<'_>) -> Vec<Call> {
    vec![Call::new(&["version"])]
}

/// Returns the hint that a refused command line offers as its recovery.
#[expect(
    clippy::unnecessary_wraps,
    reason = "a refusal hint function returns an `Option`"
)]
fn id_hint(_: &RefusalContext<'_>) -> Option<String> {
    Some("name a book by its id".to_owned())
}

/// The tool's code for crumbkit's usage code.
const USAGE: Code = Code::new("shelf", "cli", "usage");

/// A guide that declares a rename, an exit for the renamed usage code,
/// refusal alternatives and a refusal hint for every command line, and a
/// leaf that passes on its program's exit code and offers its own
/// alternatives and hint when its command line is refused.
const DECLARING: Guide = Guide::new()
    .codes(&[Rename::new(&CLI_USAGE, &USAGE)])
    .exits(&[Exit::new(64, "The command line was refused.").failures(&[&USAGE])])
    .refusal_alternatives(version_call)
    .refusal_hint(id_hint)
    .leaves(&[Leaf::new(&["books", "show"])
        .passes_on_exit("the pager")
        .refusal_alternatives(version_call)
        .refusal_hint(id_hint)]);

#[test]
fn the_report_lists_declarations_under_customisations_without_a_reason_or_a_cost() {
    let dir = TempDir::new("opt-outs-customisations").unwrap();

    run_guided(&DECLARING, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n\
         \n\
         shelf declares 7 customisations, which need no reason.\n\
         \n\
         code-rename: every run of shelf\n\
         \x20 Declares: Runs print shelf::cli::usage in place of shelf::cli::usage.\n\
         \n\
         exit-code: every run of shelf\n\
         \x20 Declares: A failure with shelf::cli::usage exits with 64, where the default is 2.\n\
         \n\
         refusal-alternatives: every run of shelf\n\
         \x20 Declares: A refused command line for a command without its own function also offers the tool's own alternatives.\n\
         \n\
         refusal-hint: every run of shelf\n\
         \x20 Declares: A refused command line for a command without its own function may recover with the tool's own hint.\n\
         \n\
         passes-on-exit: shelf books show\n\
         \x20 Declares: The command exits with the exit code of the pager.\n\
         \n\
         refusal-alternatives: shelf books show\n\
         \x20 Declares: A refused command line for this command also offers the tool's own alternatives.\n\
         \n\
         refusal-hint: shelf books show\n\
         \x20 Declares: A refused command line for this command may recover with the tool's own hint.\n"
    );
    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(report["data"], json!([]));
    assert_eq!(
        report["customisations"][4],
        json!({
            "declaration": "passes-on-exit",
            "applies_to": "shelf books show",
            "declares": "The command exits with the exit code of the pager.",
        })
    );
    assert_eq!(report["customisations"].as_array().unwrap().len(), 7);
}

#[test]
fn a_report_without_declarations_leaves_out_customisations() {
    let dir = TempDir::new("opt-outs-no-customisations").unwrap();

    run_guided(&Guide::new(), &dir, Action::Overwrite);

    let report: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(report, json!({ "schema": "shelf/opt-outs/v1", "data": [] }));
}

#[test]
fn a_leaf_that_prints_no_result_adds_nothing_to_the_report() {
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "show"]).prints_no_result()]);
    let dir = TempDir::new("opt-outs-no-result").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n"
    );
}

#[test]
fn an_exit_that_lists_the_tool_s_own_code_is_no_customisation() {
    const MINE: Code = Code::new("shelf", "books", "not_found");
    const GUIDE: Guide = Guide::new().exits(&[Exit::new(3, "No such book.").failures(&[&MINE])]);
    let dir = TempDir::new("opt-outs-own-exit").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n"
    );
}

const BOOK_SHOW: Document<Prose<String>> = Document::new("shelf", "book-show", 1, "One book.");
const VERSION: Document<Prose<String>> = Document::new("shelf", "version", 1, "The version.");

#[test]
fn a_document_named_other_than_its_command_path_is_a_document_name_customisation() {
    const GUIDE: Guide = Guide::new().leaves(&[
        Leaf::new(&["books", "show"]).document(BOOK_SHOW),
        Leaf::new(&["version"]).document(VERSION),
        Leaf::new(&["meta", "schemas"]).document(SCHEMAS_LIST),
    ]);
    let dir = TempDir::new("opt-outs-document-name").unwrap();

    run_guided(&GUIDE, &dir, Action::Overwrite);

    assert_eq!(
        read(&dir, "opt-outs.txt"),
        "shelf has no opt-outs from the conventions.\n\
         \n\
         shelf declares 1 customisation, which needs no reason.\n\
         \n\
         document-name: shelf books show\n\
         \x20 Declares: The command prints shelf/book-show/v1, where its path gives the name books-show.\n"
    );
}

#[test]
fn a_cargo_subcommand_names_its_documents_after_the_path_below_its_own_name() {
    const WEIGH: Document<Prose<String>> = Document::new("crumbkit", "weigh", 1, "The weights.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["crumbkit", "weigh"]).document(WEIGH)]);

    assert!(customisations(&Command::new("cargo"), &GUIDE).is_empty());
}

#[test]
fn the_dashed_forms_declaration_and_each_command_named_in_another_style_are_customisations() {
    let shelf = Command::new("shelf").subcommand(
        Command::new("reading_lists")
            .subcommand(Command::new("ExportAll"))
            .subcommand(Command::new("list")),
    );
    let listed = |guide: &Guide| -> Vec<_> {
        customisations(&shelf, guide)
            .into_iter()
            .map(|entry| (entry.declaration, entry.applies_to, entry.declares))
            .collect()
    };

    assert_eq!(
        listed(&Guide::new().dashed_forms()),
        [
            (
                "dashed-forms",
                "every run of shelf".to_owned(),
                "A command named in another style also answers to its dashed form.".to_owned()
            ),
            (
                "resource-name",
                "shelf reading_lists".to_owned(),
                "The command also answers to reading-lists.".to_owned()
            ),
            (
                "verb-name",
                "shelf reading_lists ExportAll".to_owned(),
                "The command also answers to export-all.".to_owned()
            ),
        ]
    );
    assert_eq!(listed(&Guide::new()), Vec::new());
}

#[test]
fn a_leaf_s_rename_and_an_exit_that_lists_it_are_customisations_of_that_command() {
    const MOVED: Code = Code::new("shelf", "books", "moved");
    const GUIDE: Guide = Guide::new()
        .exits(&[Exit::new(6, "A book moved during the import.").failures(&[&MOVED])])
        .leaves(&[Leaf::new(&["books", "import"])
            .codes(&[Rename::new(&crumbkit::codes::VERSION_CONFLICT, &MOVED)])]);
    let shelf = Command::new("shelf");

    let listed: Vec<_> = customisations(&shelf, &GUIDE)
        .into_iter()
        .map(|entry| (entry.declaration, entry.applies_to, entry.declares))
        .collect();

    assert_eq!(
        listed,
        [
            (
                "exit-code",
                "shelf books import".to_owned(),
                "A failure with shelf::books::moved exits with 6, where the default is 1."
                    .to_owned()
            ),
            (
                "code-rename",
                "shelf books import".to_owned(),
                "Runs print shelf::books::moved in place of shelf::version::conflict.".to_owned()
            ),
        ]
    );
}

/// A removed book whose text is its own line.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Removed {
    /// The book's id.
    id: u32,
}

impl crumbkit::Row for Removed {
    const COLUMNS: &'static [&'static str] = &["id"];
    const LINE: Option<&'static str> = Some("removed <ID>");
}

#[test]
fn rows_that_print_their_own_text_line_are_a_text_line_customisation() {
    const BOOKS_REMOVE: Document<Vec<Removed>> =
        Document::new("shelf", "books-remove", 1, "The removed books.");
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["books", "remove"]).document(BOOKS_REMOVE)]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let list = serde_json::to_value(&list).unwrap();
    assert_eq!(
        list,
        json!([{
            "declaration": "text-line",
            "applies_to": "shelf books remove",
            "declares": "The text prints each row as the line removed <ID> in place of a table row.",
        }])
    );
}

/// A removed shelf whose books print under it.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct RemovedShelf {
    /// The shelf's id.
    id: u32,
    /// The titles of the books on the shelf.
    books: Vec<String>,
}

impl crumbkit::Row for RemovedShelf {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("books");
}

#[test]
fn rows_that_print_their_children_under_them_are_a_children_customisation() {
    const SHELVES_REMOVE: Document<Vec<RemovedShelf>> =
        Document::new("shelf", "shelves-remove", 1, "The removed shelves.");
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["shelves", "remove"]).document(SHELVES_REMOVE)]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let list = serde_json::to_value(&list).unwrap();
    assert_eq!(
        list,
        json!([{
            "declaration": "children",
            "applies_to": "shelf shelves remove",
            "declares": "The text prints one indented line under each row for each of its books.",
        }])
    );
}

/// A removed shelf whose books print at column 1 under it.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct FlushShelf {
    /// The shelf's id.
    id: u32,
    /// The titles of the books on the shelf.
    books: Vec<String>,
}

impl crumbkit::Row for FlushShelf {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("books");
    const CHILD_INDENT: u8 = 0;
}

/// A removed shelf whose books print under it, folded into one line when
/// the shelf held more than ten, which its help says.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct FoldedShelf {
    /// The shelf's id.
    id: u32,
    /// The titles of the books on the shelf.
    books: Vec<String>,
}

impl crumbkit::Row for FoldedShelf {
    const COLUMNS: &'static [&'static str] = &["id"];
    const CHILDREN: Option<&'static str> = Some("books");
    const CHILDREN_HELP: Option<&'static str> =
        Some("one line for each book, or one folded line for more than ten books.");
}

#[test]
fn the_children_customisation_of_rows_at_column_1_does_not_call_them_indented() {
    const SHELVES_REMOVE: Document<Vec<FlushShelf>> =
        Document::new("shelf", "shelves-remove", 1, "The removed shelves.");
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["shelves", "remove"]).document(SHELVES_REMOVE)]);

    let list = serde_json::to_value(customisations(&Command::new("shelf"), &GUIDE)).unwrap();

    assert_eq!(
        list[0]["declares"],
        json!("The text prints one line under each row for each of its books.")
    );
}

#[test]
fn the_children_customisation_quotes_the_row_type_s_own_help_text() {
    const SHELVES_REMOVE: Document<Vec<FoldedShelf>> =
        Document::new("shelf", "shelves-remove", 1, "The removed shelves.");
    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["shelves", "remove"]).document(SHELVES_REMOVE)]);

    let list = serde_json::to_value(customisations(&Command::new("shelf"), &GUIDE)).unwrap();

    assert_eq!(
        list[0]["declares"],
        json!(
            "The text prints lines under each row for its books, and help describes them as: one line for each book, or one folded line for more than ten books."
        )
    );
}

/// A book that prints whole in JSON and as its own line in text.
#[derive(serde::Serialize, schemars::JsonSchema)]
struct WholeLined {
    /// The book's id.
    id: u32,
}

impl crumbkit::Row for WholeLined {
    const COLUMNS: &'static [&'static str] = &["id"];
    const WHOLE: bool = true;
    const LINE: Option<&'static str> = Some("book <ID>");
}

#[test]
fn the_report_lists_the_rows_whole_opt_out_and_the_text_line_customisation_of_one_leaf() {
    const BOOKS_SHOW: Document<Vec<WholeLined>> =
        Document::new("shelf", "books-show", 1, "The books.");
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "show"]).document(BOOKS_SHOW)]);
    const ALLOW: &[crate::Allow] = &[crate::Allow::new(
        crate::Rule::RowsWhole,
        "books show",
        "Scripts read every field of each book.",
    )];
    let dir = TempDir::new("opt-outs-whole-lined").unwrap();

    super::super::check(
        &super::shelf(),
        &GUIDE,
        ALLOW,
        &crate::Budgets::new(),
        None,
        dir.path(),
        &snapbox::Assert::new().action(Action::Overwrite),
    );

    let text = read(&dir, "opt-outs.txt");
    assert!(
        text.contains("documents/rows-whole: shelf books show\n"),
        "{text}"
    );
    assert!(text.contains("text-line: shelf books show\n"), "{text}");
    let json: Value = serde_json::from_str(&read(&dir, "opt-outs.json")).unwrap();
    assert_eq!(json["data"][0]["rule"], json!("documents/rows-whole"));
    assert_eq!(json["customisations"][0]["declaration"], json!("text-line"));
}

/// Joins the segments of a code with `.`, a custom spelling.
const DOTS: crumbkit::Spelling = crumbkit::Spelling::custom(|code, f| {
    write!(f, "{}.{}.{}", code.component(), code.area(), code.reason())
});

#[test]
fn a_custom_spelling_is_a_customisation_and_spells_every_listed_code() {
    const GUIDE: Guide = Guide::new()
        .spelling(DOTS)
        .codes(&[Rename::new(&CLI_USAGE, &USAGE)]);
    let shelf = Command::new("shelf");

    let listed: Vec<_> = customisations(&shelf, &GUIDE)
        .into_iter()
        .map(|entry| (entry.declaration, entry.applies_to, entry.declares))
        .collect();

    assert_eq!(
        listed,
        [
            (
                "code-spelling",
                "every run of shelf".to_owned(),
                "Runs print codes such as shelf.cli.usage in place of shelf::cli::usage."
                    .to_owned()
            ),
            (
                "code-rename",
                "every run of shelf".to_owned(),
                "Runs print shelf.cli.usage in place of shelf.cli.usage.".to_owned()
            ),
        ]
    );
}

#[test]
fn a_guide_that_sets_the_colon_spelling_lists_no_spelling() {
    const GUIDE: Guide = Guide::new().spelling(crumbkit::Spelling::COLONS);

    assert!(customisations(&Command::new("shelf"), &GUIDE).is_empty());
}

#[test]
fn each_example_that_needs_a_precondition_is_an_example_precondition_customisation() {
    use crumbkit::help::Example;

    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["books", "list"]).examples(&[
        Example::new(&[], "Print the first page of books"),
        Example::new(&["--all"], "Print every book from the server").needs(&["server"]),
        Example::new(&["--author", "Jane Austen"], "Compare two reports")
            .needs(&["server", "reports"]),
    ])]);

    let list = customisations(&Command::new("shelf"), &GUIDE);

    let list = serde_json::to_value(&list).unwrap();
    assert_eq!(
        list,
        json!([
            {
                "declaration": "example-precondition",
                "applies_to": "shelf books list --all",
                "declares": "The example runs only in a test run that provides server.",
            },
            {
                "declaration": "example-precondition",
                "applies_to": "shelf books list --author 'Jane Austen'",
                "declares": "The example runs only in a test run that provides server and reports.",
            },
        ])
    );
}
