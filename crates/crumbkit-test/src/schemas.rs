//! The schema golden check, which pins the JSON Schema of each document
//! that a tool prints in golden files.

use std::path::Path;

use clap::Command;
use crumbkit::help::Guide;
use snapbox::Assert;
use snapbox::assert::{Action, DEFAULT_ACTION_ENV};

use crate::goldens::{compare, remove_stale};
use crate::tree;

#[cfg(test)]
mod tests;

/// Checks that the JSON Schema of each document that the tool `command`
/// prints matches its golden file in `dir`, and panics, as a failed test
/// assertion does, naming every problem it finds.
///
/// The documents are those that `crumbkit::schemas::documents` serves for
/// `command` and `guide`, which `schemas show` prints: the document of each
/// leaf of `guide` whose command is visible, and each file format that
/// `Guide::documents` registers. The check skips the documents
/// of the component `crumbkit`, such as `crumbkit/failure/v1` and
/// `crumbkit/commands-list/v1`, because crumbkit's own tests pin their
/// schemas. The golden file of a document is named after its id, such as
/// `shelf-books-list-v1.json` for `shelf/books-list/v1`, and holds the
/// schema as pretty-printed JSON. A changed golden file shows a change to a
/// document's shape in review, where ADR-0007 decides whether the change
/// needs a new version.
///
/// `dir` holds only schema golden files, because the check counts every
/// `.json` file in it that no document produces as stale. A relative `dir`,
/// such as `tests/golden/schemas`, names a directory in the package that
/// runs the test.
///
/// The check compares each schema with snapbox, as other golden files are
/// compared (ADR-0004). With the environment variable `SNAPSHOTS=overwrite`,
/// it writes each missing file, rewrites each file whose content differs
/// and deletes each stale file. It finds two leaves that declare one id
/// with different schemas, naming both command lines, because
/// `schemas show` serves only the schema of the first. It finds two ids
/// whose golden files have one name, such as `shelf/books-list/v1` and
/// `shelf-books/list/v1`, which a component with dashes allows, naming
/// both ids, and compares or writes only the first of the two. It finds a
/// file format whose id is also the id of a leaf's document or of an
/// earlier file format, because `schemas show` serves only the first.
/// Without `SNAPSHOTS=overwrite`, it also finds:
///
/// - a document without a golden file;
/// - a schema that differs from its golden file, with snapbox's report of
///   both texts;
/// - a stale golden file: a `.json` file in `dir` that no document
///   produces, such as the schema of a removed command's document.
///
/// ```no_run
/// use clap::Command;
/// use crumbkit::help::Guide;
///
/// let shelf = Command::new("shelf");
///
/// crumbkit_test::check_schema_goldens(&shelf, &Guide::new(), "tests/golden/schemas");
/// ```
///
/// # Panics
///
/// Panics when a document or a golden file breaks a rule above, or when
/// `SNAPSHOTS=overwrite` cannot write or delete a file. Each problem in the
/// message starts on a new line and names the document by its id, such as
/// `shelf/books-list/v1`.
pub fn check_schema_goldens(command: &Command, guide: &Guide, dir: impl AsRef<Path>) {
    let assert = Assert::new().action_env(DEFAULT_ACTION_ENV);
    check(command, guide, dir.as_ref(), &assert);
}

/// Runs [`check_schema_goldens`] with `assert`, whose action decides
/// whether the check compares or overwrites, so a test chooses the action
/// without setting `SNAPSHOTS`.
fn check(command: &Command, guide: &Guide, dir: &Path, assert: &Assert) {
    let overwrite = assert.selected_action() == Action::Overwrite;
    let mut problems = Vec::new();
    find_conflicts(command, guide, &mut problems);
    // Each golden file name with the id of the document that has it.
    let mut names: Vec<(String, String)> = Vec::new();
    for served in crumbkit::schemas::documents(command, guide) {
        let document = served.document();
        if document.component() == "crumbkit" {
            continue;
        }
        let name = format!(
            "{}-{}-v{}.json",
            document.component(),
            document.name(),
            document.version()
        );
        let id = served.id();
        if let Some((_, other)) = names.iter().find(|(known, _)| *known == name) {
            // `documents` serves the documents of leaves once for each id,
            // so a second entry with the same id is a file format.
            if *other == id {
                problems.push(format!(
                    "{id}: a file format has the id of another document, so one of the two needs another id"
                ));
            } else {
                problems.push(format!(
                    "{id}: the golden file name `{name}` is also the name for the document {other}, so one of the two needs another id"
                ));
            }
            continue;
        }
        // Serializing a `Schema`, which holds a `serde_json::Value`, cannot
        // fail, and an empty text would show as a differing file.
        let text = serde_json::to_string_pretty(&served.schema()).unwrap_or_default() + "\n";
        compare(assert, &id, "schema", text, &dir.join(&name), &mut problems);
        names.push((name, id));
    }
    let names: Vec<String> = names.into_iter().map(|(name, _)| name).collect();
    remove_stale(dir, &names, "json", overwrite, "document", &mut problems);
    tree::report("golden schema rules", &problems);
}

/// Adds a problem to `problems` for each pair of leaves of `guide` that
/// declare one document id with different schemas, naming both command
/// lines below `command`.
fn find_conflicts(command: &Command, guide: &Guide, problems: &mut Vec<String>) {
    let leaves: Vec<_> = guide
        .get_leaves()
        .iter()
        .filter_map(|leaf| Some((leaf, leaf.get_document()?.to_string())))
        .collect();
    for (index, (first, id)) in leaves.iter().enumerate() {
        for (second, other) in &leaves[index + 1..] {
            if id != other || first.document_schema() == second.document_schema() {
                continue;
            }
            problems.push(format!(
                "{id}: `{}` and `{}` print the document with different schemas, so one of them needs a document of its own",
                command_line(command, first.path()),
                command_line(command, second.path())
            ));
        }
    }
}

/// Returns the command line of `path` below `command`, such as
/// `shelf books list`.
pub(crate) fn command_line(command: &Command, path: &[&str]) -> String {
    std::iter::once(command.get_name())
        .chain(path.iter().copied())
        .collect::<Vec<_>>()
        .join(" ")
}
