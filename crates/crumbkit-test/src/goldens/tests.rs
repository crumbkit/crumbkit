//! Tests for the help golden check.

use std::fs;

use clap::Command;
use crumbkit::Tool;
use snapbox::Assert;
use snapbox::assert::Action;

use super::check;
use crate::pages::Pages;
use crate::temp::TempDir;
use crate::testing::panic_message;

/// A tool with one group and one leaf.
fn shelf() -> Command {
    Command::new("shelf")
        .about("Browse a small catalogue")
        .subcommand(
            Command::new("books")
                .about("Work with books")
                .subcommand(Command::new("list").about("List the books")),
        )
}

/// The golden files of `shelf()`, sorted by name.
const FILES: [&str; 6] = [
    "shelf-books-h.txt",
    "shelf-books-help.txt",
    "shelf-books-list-h.txt",
    "shelf-books-list-help.txt",
    "shelf-h.txt",
    "shelf-help.txt",
];

/// Runs the check on `shelf()` against `dir` with `action`.
fn run(dir: &TempDir, action: Action) {
    check(
        &shelf(),
        Pages::rendered(&Tool::new()),
        dir.path(),
        &Assert::new().action(action),
    );
}

#[test]
fn overwrite_writes_a_golden_file_for_each_page_and_deletes_a_stale_one() {
    let dir = TempDir::new("goldens-overwrite").unwrap();
    dir.write("shelf-authors-h.txt", "Work with authors\n");
    dir.write("notes.md", "Only .txt files are golden files.\n");

    run(&dir, Action::Overwrite);

    let mut expected: Vec<&str> = FILES.to_vec();
    expected.insert(0, "notes.md");
    assert_eq!(dir.names(), expected);
    let page = fs::read_to_string(dir.path().join("shelf-books-list-h.txt")).unwrap();
    assert!(page.starts_with("List the books\n\nUsage:"), "{page}");
}

#[test]
fn golden_files_that_hold_every_page_pass() {
    let dir = TempDir::new("goldens-pass").unwrap();
    run(&dir, Action::Overwrite);

    run(&dir, Action::Verify);
}

#[test]
fn a_page_without_a_golden_file_fails_the_check() {
    let dir = TempDir::new("goldens-missing").unwrap();
    let file = |name: &str| dir.path().join(name).display().to_string();

    assert_eq!(
        panic_message(|| run(&dir, Action::Verify)).unwrap(),
        format!(
            "the command tree breaks crumbkit's golden help rules:\n\
             shelf -h: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books -h: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books list -h: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf --help: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books --help: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it\n\
             shelf books list --help: the golden file `{}` is missing, and `SNAPSHOTS=overwrite` writes it",
            file("shelf-h.txt"),
            file("shelf-books-h.txt"),
            file("shelf-books-list-h.txt"),
            file("shelf-help.txt"),
            file("shelf-books-help.txt"),
            file("shelf-books-list-help.txt"),
        )
    );
}

#[test]
fn a_golden_file_whose_content_differs_fails_the_check() {
    let dir = TempDir::new("goldens-differs").unwrap();
    run(&dir, Action::Overwrite);
    dir.write("shelf-books-h.txt", "Work with the books\n");

    let message = panic_message(|| run(&dir, Action::Verify)).unwrap();

    let heading = format!(
        "the command tree breaks crumbkit's golden help rules:\n\
         shelf books -h: the page differs from the golden file `{}`:\n",
        dir.path().join("shelf-books-h.txt").display()
    );
    assert!(message.starts_with(&heading), "{message}");
    assert!(message.contains("Work with the books"), "{message}");
    assert_eq!(
        message.matches(": the page differs").count(),
        1,
        "{message}"
    );
}

#[test]
fn a_stale_golden_file_fails_the_check() {
    let dir = TempDir::new("goldens-stale").unwrap();
    run(&dir, Action::Overwrite);
    dir.write("shelf-authors-h.txt", "Work with authors\n");

    assert_eq!(
        panic_message(|| run(&dir, Action::Verify)).unwrap(),
        format!(
            "the command tree breaks crumbkit's golden help rules:\n\
             the golden file `{}` belongs to no help page, and `SNAPSHOTS=overwrite` deletes it",
            dir.path().join("shelf-authors-h.txt").display()
        )
    );
}

#[test]
fn two_pages_with_one_golden_file_name_fail_the_check_naming_both_command_lines() {
    let twins = shelf().subcommand(Command::new("books-list").about("List the books again"));
    let dir = TempDir::new("goldens-collision").unwrap();

    assert_eq!(
        panic_message(|| check(
            &twins,
            Pages::rendered(&Tool::new()),
            dir.path(),
            &Assert::new().action(Action::Overwrite)
        ))
        .unwrap(),
        "the command tree breaks crumbkit's golden help rules:\n\
         shelf books-list -h: the page's golden file name `shelf-books-list-h.txt` is also the name for the page `shelf books list -h`\n\
         shelf books-list --help: the page's golden file name `shelf-books-list-help.txt` is also the name for the page `shelf books list --help`"
    );
}

#[cfg(unix)]
#[test]
fn a_binary_s_pages_are_compared_with_the_golden_files() {
    let dir = TempDir::new("goldens-binary").unwrap();
    let tool_name = TempDir::new("goldens-binary-tool").unwrap();
    let binary = crate::testing::help_binary(
        &tool_name,
        &[
            ("-h", "shelf -h\n"),
            ("--help", "shelf --help\n"),
            ("books -h", "books -h\n"),
            ("books --help", "books --help\n"),
            ("books list -h", "list -h\n"),
            ("books list --help", "list --help\n"),
        ],
    );
    let examples = crate::Examples::new(&binary, "");
    let tool = Tool::new();
    let pages = Pages::binary(&tool, &examples);

    check(
        &shelf(),
        pages,
        dir.path(),
        &Assert::new().action(Action::Overwrite),
    );

    assert_eq!(dir.names(), FILES);
    let page = fs::read_to_string(dir.path().join("shelf-books-list-h.txt")).unwrap();
    assert_eq!(page, "list -h\n");
    check(
        &shelf(),
        pages,
        dir.path(),
        &Assert::new().action(Action::Verify),
    );
}

/// A tree like `shelf()` whose root page holds `about`, so two texts of
/// different lengths make the same golden file names with different pages.
fn shelf_about(about: &str) -> Command {
    Command::new("shelf")
        .about(about.to_owned())
        .subcommand(Command::new("books").about("Work with books"))
}

/// Overwrites the help goldens of `command` into `dir`.
fn overwrite(command: &Command, dir: &std::path::Path) {
    check(
        command,
        Pages::rendered(&Tool::new()),
        dir,
        &Assert::new().action(Action::Overwrite),
    );
}

#[test]
fn overwriting_one_directory_from_several_threads_never_shows_a_reader_a_partial_golden_file() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let short = shelf_about(&"a short page. ".repeat(4_000));
    let long = shelf_about(&"a longer page. ".repeat(40_000));
    // The complete text of each golden file under each of the two trees.
    let mut complete: Vec<(String, Vec<String>)> = Vec::new();
    for command in [&short, &long] {
        let alone = TempDir::new("goldens-parallel-alone").unwrap();
        overwrite(command, alone.path());
        for name in alone.names() {
            let text = fs::read_to_string(alone.path().join(&name)).unwrap();
            match complete.iter_mut().find(|(known, _)| *known == name) {
                Some((_, texts)) => texts.push(text),
                None => complete.push((name, vec![text])),
            }
        }
    }
    let dir = TempDir::new("goldens-parallel").unwrap();
    overwrite(&short, dir.path());
    let done = AtomicBool::new(false);

    let partial = std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            let mut partial = Vec::new();
            while !done.load(Ordering::Relaxed) {
                for (name, texts) in &complete {
                    let Ok(text) = fs::read_to_string(dir.path().join(name)) else {
                        partial.push(format!("{name} is missing"));
                        continue;
                    };
                    if !texts.contains(&text) {
                        partial.push(format!("{name} holds {} bytes of neither page", text.len()));
                    }
                }
            }
            partial
        });
        let writers: Vec<_> = (0..4)
            .map(|index| {
                let (short, long, dir) = (&short, &long, dir.path());
                scope.spawn(move || {
                    for round in 0..15 {
                        overwrite(
                            if (round + index) % 2 == 0 {
                                long
                            } else {
                                short
                            },
                            dir,
                        );
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        done.store(true, Ordering::Relaxed);
        reader.join().unwrap()
    });

    assert_eq!(partial, Vec::<String>::new());
}

#[test]
fn a_temporary_file_that_a_crashed_overwrite_left_is_neither_stale_nor_deleted() {
    let dir = TempDir::new("goldens-leftover").unwrap();
    dir.write(".shelf-h.txt.1.0.tmp", "half a page");

    run(&dir, Action::Overwrite);
    run(&dir, Action::Verify);

    assert_eq!(
        dir.names().first().map(String::as_str),
        Some(".shelf-h.txt.1.0.tmp")
    );
}
