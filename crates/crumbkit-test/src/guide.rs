//! The guide check, which keeps a tool's goal headings, leaves and help
//! topics in step with its command tree.

use clap::parser::ValueSource;
use clap::{ArgMatches, Command};
use crumbkit::Shape;
use crumbkit::help::{Column, Guide, Leaf, ViewFlag, quote};

use crate::allow::{Allow, Allowances, Rule};
use crate::tree::{self, DASHED_NAME};

mod children;
#[cfg(test)]
mod tests;

/// Checks that `guide` names the commands of the tree of `command` as
/// crumbkit's goal headings list them and as its leaves describe them, and
/// panics, as a failed test assertion does, naming every problem it finds.
///
/// `crumbkit::parse` and `crumbkit::help::extend` never fail on a guide at
/// runtime: they skip a group, a leaf or a name that matches no command,
/// and list a visible subcommand that no goal names under a final
/// `Commands` heading. This check catches each of those cases in a test
/// instead. A group's path
/// runs from `command`, and the check reads the tree as the tool built it,
/// so clap's generated `help` subcommand is never a child. It finds:
///
/// - a root, set with `Guide::root`, whose path names no command, matching
///   real names only, because `parse` then answers topics at the clap root;
/// - a group whose path names no command, matching real names only;
/// - a group whose path names a command without subcommands;
/// - a goal that names a command that is not a subcommand of its group,
///   such as an alias or a misspelt name;
/// - a visible subcommand of a group that no goal names;
/// - a subcommand that two goals of its group name, or that one goal names
///   twice;
/// - two groups with one path, because each group replaces the goal
///   headings of the one before it, so help shows only the last;
/// - a leaf whose path names no command, matching real names only;
/// - a leaf whose path names a command with subcommands;
/// - two leaves with one path, because `parse` applies only the first;
/// - an example that clap refuses when it parses the tool's name, the
///   leaf's path and the example's words, naming clap's error kind and the
///   example's command line as help prints it, with each word that a POSIX
///   shell would split or expand between single quotes. An
///   example with `--help` or `--version` counts as refused, because an
///   example shows a real run;
/// - an example that types `--fields`, `--long` or `--full` when the leaf
///   does not offer that flag (`Leaf::offers`), because crumbkit refuses
///   the run;
/// - a visible command without subcommands, the root included when it has
///   none, that no leaf with at least one example names, because its help
///   then shows no example, unless an opt-out from [`Rule::GuideExamples`]
///   in `allow` covers it;
/// - a leaf whose [`Effects`](crumbkit::help::Effects) change something,
///   whose command takes a view flag and that declares no document with
///   `Leaf::document` or `Leaf::stream`, because the leaf then offers
///   every view flag, and only `finish` or `stream` refuses a flag that
///   the output cannot use, after the handler has made its change
///   (ADR-0016), unless an opt-out from [`Rule::GuideDocument`] in `allow`
///   covers it. A command takes a view flag when it reaches a global
///   argument with the id `json`, `fields`, `long` or `full`, which is how
///   `parse` recognises one (ADR-0115). A read-only leaf, a leaf without
///   effects and a leaf whose command takes no view flag pass without a
///   document. So does a leaf that passes on a program's exit code
///   (`Leaf::passes_on_exit`), because `crumbkit_child::hand_over` refuses
///   `--json` and `--fields` before the program starts;
/// - a leaf's [`Choices`](crumbkit::pick::Choices) whose argument id names
///   no argument of the leaf's command, because a picker then never offers
///   them;
/// - a leaf column whose name is not a field of the leaf's row type, such
///   as a misspelt name in `Row::COLUMNS` or `Row::LONG_COLUMNS`, because
///   the table prints `-` in every row of it;
/// - a headed leaf (`Document::headed`) whose row type has no top-level
///   `text` or `base64` field, because its JSON then holds the bytes
///   under another field; the row marks its `Bytes` field
///   `#[serde(flatten)]` (ADR-0062);
/// - a leaf whose document holds rows and is not headed, and whose row
///   type has no `Row::COLUMNS`, because its table and its JSON without
///   `--full` then print nothing of each row. `finish` and `stream` already
///   fail to compile for such a row type in every result but a batch
///   (ADR-0063). A row type that sets `Row::WHOLE` and names no columns in
///   either list is exempt, because its text prints each row as one line
///   of JSON and its JSON holds every field;
/// - a leaf whose row type sets `Row::WHOLE`, names no columns in either
///   list and has fields in its JSON Schema, because those fields could be
///   the columns of a table (ADR-0065);
/// - a leaf whose row type's `Row::LINE` is not one non-empty line,
///   because help shows the form on one line, and a headed leaf whose row
///   type sets `Row::LINE`, because a headed batch's text prints each row's
///   bytes and never the line (ADR-0064);
/// - a leaf whose row type's `Row::CHILDREN` names a field that is not a
///   field of the row type, has no doc comment, or is also one of its
///   `Row::COLUMNS` or `Row::LONG_COLUMNS`, and a headed leaf whose row
///   type sets `Row::CHILDREN`, because a headed batch's text never prints
///   the child lines (ADR-0120);
/// - a leaf column without a description, naming the field whose doc
///   comment is missing, because `--help` lists the column without one;
/// - a leaf whose row type's `Row::PROBLEM` names a field that is not one
///   of its `Row::COLUMNS`, because the compact table then hides the
///   problem that makes the run exit with 1;
/// - a leaf whose problem field's type is not a string, a boolean or a
///   value that can be null, because no value of it then means that the
///   row has no problem;
/// - a topic whose name is the name or an alias of a command directly under
///   `command`, because `<tool> help <name>` prints that command's help;
/// - a topic whose text is empty or only whitespace;
/// - a topic whose name an earlier topic already has, because `parse`
///   prints only the first;
/// - a topic name that is not lowercase ASCII letters and digits in words
///   joined by single dashes, starting with a letter, such as `catalogue`.
///
/// A hidden subcommand passes whether or not a goal names it, and a hidden
/// command needs no leaf. A command that no group names keeps clap's list
/// and is not checked. The column rules read
/// [`Leaf::column_descriptions`](crumbkit::help::Leaf::column_descriptions),
/// the data that the leaf's `--help` lists. A topic with
/// an empty summary passes.
///
/// Two rules take an opt-out, an [`Allow`] with a stated reason, which
/// covers the command it names and every command below it
/// ([opt-outs docs]):
///
/// - [`Rule::GuideExamples`] excuses the example rule, so a tool that
///   adopts goal headings and leaves before it has written an example for
///   every command passes the check, and the reason says when the examples
///   come;
/// - [`Rule::GuideDocument`] excuses the document rule, so a tool that
///   prints its own output and uses crumbkit for help can declare the
///   effects of a leaf that changes something before its output moves
///   onto crumbkit's documents. crumbkit then cannot refuse a view flag
///   that the output cannot use before the change happens, so the tool's
///   own output decides what the flag does.
///
/// The check fails on such an opt-out that names no command or excuses no
/// problem, so the opt-out goes once the command has an example or a
/// document. The other rules take no opt-out, because a tool that wants
/// clap's list for a command leaves that command out of the guide, and
/// each other problem is a name or a declaration that the tool can
/// correct. The check reads only the opt-outs of its own rules, so a tool
/// passes the one list that it passes to every check.
///
/// ```
/// use clap::Command;
/// use crumbkit::help::{Example, Goal, Group, Guide, Leaf, Topic};
///
/// const GUIDE: Guide = Guide::new()
///     .groups(&[Group::new(
///         &["books"],
///         &[Goal::new("Read", &["list", "show"])],
///     )])
///     .leaves(&[
///         Leaf::new(&["books", "list"]).examples(&[Example::new(&[], "Print the books")]),
///         Leaf::new(&["books", "show"]).examples(&[Example::new(&[], "Show the first book")]),
///     ])
///     .topics(&[Topic::new(
///         "catalogue",
///         "Where the books come from",
///         "The catalogue is built into the binary.\n",
///     )]);
///
/// let shelf = Command::new("shelf").subcommand(
///     Command::new("books")
///         .subcommand(Command::new("list"))
///         .subcommand(Command::new("show")),
/// );
///
/// crumbkit_test::check_guide(&shelf, &GUIDE, &[]);
/// ```
///
/// # Panics
///
/// Panics when the guide breaks a rule above, or when an opt-out from
/// [`Rule::GuideExamples`] or [`Rule::GuideDocument`] names no command or
/// excuses no problem. The
/// message lists each problem on its own line and names each command by its
/// path from `command`, such as `shelf books`.
///
/// [opt-outs docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/adoption/opt-outs.md
pub fn check_guide(command: &Command, guide: &Guide, allow: &[Allow]) {
    let mut allowances = Allowances::new(allow, &[Rule::GuideExamples, Rule::GuideDocument]);
    let mut problems = Vec::new();
    let root = guide.get_root();
    if tree::find(command, root.iter().copied()).is_none() {
        problems.push(format!("the root `{}` names no command", root.join(" ")));
    }
    let groups = guide.get_groups();
    for (index, group) in groups.iter().enumerate() {
        let path = group.path();
        let Some(node) = tree::find(command, path.iter().copied()) else {
            problems.push(format!("the group `{}` names no command", path.join(" ")));
            continue;
        };
        let text = std::iter::once(command.get_name())
            .chain(path.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        if !node.has_subcommands() {
            problems.push(format!(
                "{text}: the guide gives the command goals, and it has no subcommands to list under them"
            ));
            continue;
        }
        // Only the second group with a path reports it, so a path that
        // three groups share is one problem.
        let earlier = groups[..index]
            .iter()
            .filter(|earlier| earlier.path() == path)
            .count();
        if earlier == 1 {
            problems.push(format!(
                "{text}: two groups name the command, and help shows only the goals of the last"
            ));
        }
        for goal in group.goals() {
            for &name in goal.commands() {
                if tree::find(node, [name]).is_none() {
                    problems.push(format!(
                        "{text}: the goal `{}` names `{name}`, which is not a subcommand of the command",
                        goal.heading()
                    ));
                }
            }
        }
        for subcommand in node.get_subcommands() {
            let name = subcommand.get_name();
            let mut headings: Vec<String> = Vec::new();
            for goal in group.goals() {
                let times = goal
                    .commands()
                    .iter()
                    .filter(|&&named| named == name)
                    .count();
                if times > 0 {
                    headings.push(format!("`{}`", goal.heading()));
                }
                if times > 1 {
                    problems.push(format!(
                        "{text}: the goal `{}` names `{name}` more than once, so help lists it more than once under that heading",
                        goal.heading()
                    ));
                }
            }
            match headings.as_slice() {
                [] if !subcommand.is_hide_set() => problems.push(format!(
                    "{text}: no goal names the visible subcommand `{name}`, so help lists it under a final `Commands` heading"
                )),
                [] | [_] => {}
                [first @ .., last] => problems.push(format!(
                    "{text}: the goals {} and {last} name `{name}`, so help lists it under each of them",
                    first.join(", ")
                )),
            }
        }
    }
    check_leaves(command, guide, &mut allowances, &mut problems);
    check_topics(command, guide, &mut problems);
    allowances.report(command, "guide rules", problems);
}

/// Adds a problem to `problems` for each leaf of `guide` that breaks a leaf
/// rule of [`check_guide`], and for each visible command without
/// subcommands that no leaf with an example names and no opt-out in
/// `allowances` covers.
fn check_leaves(
    command: &Command,
    guide: &Guide,
    allowances: &mut Allowances<'_>,
    problems: &mut Vec<String>,
) {
    let tool = command.get_name();
    let leaves = guide.get_leaves();
    // clap builds the tree that it parses, so the examples parse against a
    // copy, and `command` stays as the tool built it for the other rules.
    let mut parser = command.clone();
    for (index, leaf) in leaves.iter().enumerate() {
        let path = leaf.path();
        let Some(node) = tree::find(command, path.iter().copied()) else {
            problems.push(format!("the leaf `{}` names no command", path.join(" ")));
            continue;
        };
        let text = std::iter::once(tool)
            .chain(path.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        if node.has_subcommands() {
            problems.push(format!(
                "{text}: the guide declares the command a leaf, and it has subcommands"
            ));
            continue;
        }
        // Only the second leaf with a path reports it, so a path that three
        // leaves share is one problem.
        let earlier = leaves[..index]
            .iter()
            .filter(|earlier| earlier.path() == path)
            .count();
        if earlier == 1 {
            problems.push(format!(
                "{text}: two leaves name the command, and help shows only the first"
            ));
        }
        for example in leaf.get_examples() {
            let args = std::iter::once(tool)
                .chain(path.iter().copied())
                .chain(example.words().iter().copied());
            let line = command_line(&text, example.words());
            match parser.try_get_matches_from_mut(args) {
                Ok(matches) => {
                    for flag in unoffered_flags(leaf, node, &matches) {
                        let flag = flag.name();
                        problems.push(format!(
                            "{text}: the example `{line}` types {flag}, which the command does not offer, so crumbkit refuses it"
                        ));
                    }
                }
                Err(error) => problems.push(format!(
                    "{text}: clap refuses the example `{line}` with the error kind {:?}",
                    error.kind()
                )),
            }
        }
        // A leaf that passes on a program's exit code hands the terminal
        // to it, and `crumbkit_child::hand_over` refuses `--json` and
        // `--fields` before the program starts.
        let changes = leaf
            .get_effects()
            .is_some_and(|effects| !effects.is_read_only());
        let hands_over = leaf.get_passes_on_exit().is_some();
        if changes
            && !hands_over
            && leaf.get_document().is_none()
            && takes_view_flag(command, path)
            && !allowances.excuses(Rule::GuideDocument, &path.join(" "))
        {
            problems.push(format!(
                "{text}: the leaf declares effects that change something and no document, so crumbkit refuses a view flag that the output cannot use only after the change; declare the document with `Leaf::document`, or with `Leaf::stream` for a stream"
            ));
        }
        for choices in leaf.get_choices() {
            let id = choices.arg();
            if !node.get_arguments().any(|arg| arg.get_id() == id) {
                problems.push(format!(
                    "{text}: the choices for `{id}` name no argument of the command, so a picker never offers them"
                ));
            }
        }
        check_columns(&text, leaf, problems);
        check_line(&text, leaf, problems);
        children::check_children(&text, leaf, problems);
    }
    for visit in tree::walk_visible(command) {
        let named = leaves
            .iter()
            .any(|leaf| !leaf.get_examples().is_empty() && leaf.path().join(" ") == visit.below());
        if !visit.command().has_subcommands()
            && !named
            && !allowances.excuses(Rule::GuideExamples, visit.below())
        {
            problems.push(format!(
                "{}: no leaf with an example names the command, so its help shows no example",
                visit.path()
            ));
        }
    }
}

/// Adds a problem to `problems` for each column of the leaf at `text` that
/// is not a field of its row type or has no description, for the problem
/// field that `check_problem` refuses, for a headed leaf's `text` or
/// `base64` that is not at the top level of the row type, and for rows
/// whose row type has no compact column.
fn check_columns(text: &str, leaf: &Leaf, problems: &mut Vec<String>) {
    let columns = leaf.column_descriptions();
    if let Some(field) = leaf.problem() {
        check_problem(text, field, &columns, problems);
    }
    let headed = leaf
        .get_document()
        .is_some_and(|document| document.is_headed());
    // Prose and bytes have no rows, a headed batch prints each row's bytes
    // in text and every field in JSON, and rows that print whole without
    // columns print each row's own line in text and every field in JSON,
    // so only other rows need a compact column. The compiler refuses a
    // whole row type without columns and without a `LINE` for every shape
    // but a batch, which this check covers (ADR-0072).
    let without_rows = matches!(leaf.shape(), Some(Shape::Prose | Shape::Bytes));
    let rows = leaf.get_document().is_some() && !without_rows && !headed;
    let whole_without_columns = leaf.rows_whole() && columns.is_empty();
    if rows && whole_without_columns && leaf.line().is_none() {
        problems.push(format!(
            "{text}: the row type sets Row::WHOLE and has no COLUMNS and no LINE, so its text has no form for a row; declare the row's own text line in `Row::LINE`"
        ));
    } else if rows && whole_without_columns && leaf.row_has_fields() {
        problems.push(format!(
            "{text}: the row type has fields and no COLUMNS, so its text prints each row's own line where a table could show its fields; name the fields that identify a row in `Row::COLUMNS`"
        ));
    } else if rows && !whole_without_columns && columns.iter().all(Column::is_long) {
        problems.push(format!(
            "{text}: the row type has no COLUMNS, so the table and the JSON without --full print nothing of each row; name the fields that identify a row in `Row::COLUMNS`"
        ));
    }
    for column in columns {
        let name = column.name();
        if headed && !column.is_property() && ["text", "base64"].contains(&name) {
            problems.push(format!(
                "{text}: the field `{name}` is not at the top level of the row type, so the JSON of the headed document does not hold the bytes beside the other fields; mark the `Bytes` field `#[serde(flatten)]`"
            ));
        } else if !column.is_property() {
            problems.push(format!(
                "{text}: the column `{name}` is not a field of the row type, so the table prints `-` in every row of it"
            ));
        } else if column.description().is_none() {
            problems.push(format!(
                "{text}: the column `{name}` has no description, because the field `{name}` of the row type has no doc comment"
            ));
        }
    }
}

/// Adds a problem to `problems` when the leaf at `text` declares a text
/// line form, `Row::LINE`, that is not one non-empty line, or declares one
/// for a headed document, whose text never prints it (ADR-0064).
fn check_line(text: &str, leaf: &Leaf, problems: &mut Vec<String>) {
    let Some(line) = leaf.line() else {
        return;
    };
    if line.trim().is_empty() || line.contains(['\n', '\r']) {
        problems.push(format!(
            "{text}: the row type's LINE is not one non-empty line, so help cannot show the form of each row's line on one line; write the form as one line, such as `<ID> <TITLE>`"
        ));
    }
    if leaf
        .get_document()
        .is_some_and(|document| document.is_headed())
    {
        problems.push(format!(
            "{text}: the row type declares LINE and the document is headed, so its text prints each row's bytes and never the line; remove LINE or the headed form"
        ));
    }
}

/// Adds a problem to `problems` when `field`, the field that `Row::PROBLEM`
/// names for the leaf at `text`, is not one of `columns` without `--long`,
/// or when its type has no value that means no problem.
fn check_problem(text: &str, field: &str, columns: &[Column], problems: &mut Vec<String>) {
    let column = columns
        .iter()
        .find(|column| column.name() == field && !column.is_long());
    match column {
        None => problems.push(format!(
            "{text}: the problem field `{field}` is not one of the row type's COLUMNS, so the compact table hides the problem that decides the exit"
        )),
        Some(column) if column.is_property() && !column.can_be_empty() => problems.push(format!(
            "{text}: the problem field `{field}` is not a string, a boolean or a value that can be null, so no value of it means that the row has no problem"
        )),
        Some(_) => {}
    }
}

/// Returns each view flag that the caller typed in `matches`, the matches
/// of one example of `leaf`, and that the leaf does not offer. The flag's
/// value sits in the matches of the leaf's command, at the end of the path
/// of subcommands, also when the example types it before that path. A flag
/// whose name `node`, the leaf's command, declares as its own argument that
/// is not global does not count, because it is the tool's own flag.
fn unoffered_flags(leaf: &Leaf, node: &Command, matches: &ArgMatches) -> Vec<ViewFlag> {
    let mut level = matches;
    while let Some((_, next)) = level.subcommand() {
        level = next;
    }
    // crumbkit's `ViewFlags` gives each view flag the id of its long name.
    let typed = |id: &str| {
        level.ids().any(|known| known == id)
            && level.value_source(id) == Some(ValueSource::CommandLine)
            && !node
                .get_arguments()
                .any(|arg| arg.get_id() == id && !arg.is_global_set())
    };
    [ViewFlag::Fields, ViewFlag::Long, ViewFlag::Full]
        .into_iter()
        .filter(|flag| !leaf.offers(*flag) && typed(flag.name().trim_start_matches('-')))
        .collect()
}

/// Reports whether the command at `path` below `command` takes a view
/// flag: an argument with the id `json`, `fields`, `long` or `full` that
/// is global, which `parse` reads as a view flag. The tree is unbuilt, so
/// the check walks from the leaf's command up to `command` and asks, for
/// each id, the nearest command that declares an argument with it. clap's
/// build copies a global argument down only to a command that has no
/// argument with its id, so that nearest argument is the one the leaf's
/// command parses, and it is a view flag only when it is global.
fn takes_view_flag(command: &Command, path: &[&str]) -> bool {
    let mut chain = vec![command];
    for &name in path {
        let Some(child) = tree::find(chain[chain.len() - 1], [name]) else {
            return false;
        };
        chain.push(child);
    }
    ["json", "fields", "long", "full"].into_iter().any(|id| {
        chain
            .iter()
            .rev()
            .find_map(|node| node.get_arguments().find(|arg| arg.get_id() == id))
            .is_some_and(clap::Arg::is_global_set)
    })
}

/// Returns the command line of an example as help prints it: `text`, the
/// tool's name and the leaf's path joined by spaces, followed by each of
/// `words` quoted for a POSIX shell by `crumbkit::help::quote`, such as
/// `shelf books list --author 'Jane Austen'` (ADR-0024).
pub(crate) fn command_line(text: &str, words: &[&str]) -> String {
    let mut line = text.to_owned();
    for &word in words {
        line.push(' ');
        line.push_str(&quote(word));
    }
    line
}

/// Adds a problem to `problems` for each topic of `guide` that breaks a
/// topic rule of [`check_guide`].
fn check_topics(command: &Command, guide: &Guide, problems: &mut Vec<String>) {
    let tool = command.get_name();
    let topics = guide.get_topics();
    for (index, topic) in topics.iter().enumerate() {
        let name = topic.name();
        if !crumbkit::is_dashed(name) {
            problems.push(format!("the topic name `{name}` is not {DASHED_NAME}"));
        }
        // `find_subcommand` resolves aliases, as `parse` does.
        if let Some(subcommand) = command.find_subcommand(name) {
            problems.push(format!(
                "the topic `{name}` is a name of the command `{tool} {}`, so `{tool} help {name}` prints that command's help",
                subcommand.get_name()
            ));
        }
        // crumbkit's `help` prints its own exit-codes topic before it looks
        // for a tool's topic with that name.
        if name == "exit-codes" {
            problems.push(format!(
                "the topic `{name}` has the name of crumbkit's own topic, so `{tool} help {name}` prints crumbkit's exit codes in its place"
            ));
        }
        if topic.text().trim().is_empty() {
            problems.push(format!("the topic `{name}` has no text"));
        }
        // Only the second topic with a name reports it, so a name that
        // three topics share is one problem.
        let earlier = topics[..index]
            .iter()
            .filter(|earlier| earlier.name() == name)
            .count();
        if earlier == 1 {
            problems.push(format!(
                "two topics are named `{name}`, and `{tool} help {name}` prints only the first"
            ));
        }
    }
}
