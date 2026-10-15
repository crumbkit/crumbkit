use std::path::Path;

use super::{List, lists_in};

/// Returns the name of each list in `lists`, in order.
fn names(lists: &[List]) -> Vec<&str> {
    lists.iter().map(List::name).collect()
}

#[test]
fn the_lists_of_a_directory_are_its_text_files_in_name_order() {
    let lists = lists_in(Path::new("tests/fixtures/lists"));
    assert_eq!(names(&lists), ["finished", "to-read"]);
    assert_eq!(lists[1].about(), "Books to read next");
}

#[test]
fn a_file_whose_name_is_not_dashed_or_names_a_static_command_is_not_a_list() {
    let lists = lists_in(Path::new("tests/fixtures/skipped"));
    assert_eq!(names(&lists), ["kept", "no-about"]);
}

#[test]
fn a_list_without_an_about_line_has_an_about_that_names_it() {
    let lists = lists_in(Path::new("tests/fixtures/skipped"));
    assert_eq!(lists[1].about(), "Work with the no-about reading list");
}

#[test]
fn a_directory_that_does_not_exist_holds_no_lists() {
    assert_eq!(lists_in(Path::new("tests/fixtures/missing")), []);
}

#[test]
fn a_tool_without_lists_has_the_guide_and_the_commands_of_its_static_grammar() {
    let guide = crate::guide(&[]);
    assert_eq!(guide.get_leaves().len(), crate::GUIDE.get_leaves().len());
    let command = crate::command(&[]);
    let names: Vec<&str> = command
        .get_subcommands()
        .map(clap::Command::get_name)
        .collect();
    assert_eq!(names, ["entries", "lists", "commands", "schemas"]);
}

#[test]
fn an_export_that_cannot_open_the_file_names_its_path_in_the_hint() {
    let error = super::EntriesError::CannotOpen {
        name: "to-read".to_owned(),
        path: std::path::PathBuf::from("lists/to-read.txt"),
        error: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    };

    assert_eq!(error.to_string(), "cannot read the list to-read");
    assert_eq!(crumbkit::Failure::code(&error), &super::UNREADABLE);
    assert_eq!(
        crumbkit::Failure::recovery(&error),
        crumbkit::Recovery::Hint(
            "reading could not open lists/to-read.txt, so check that the file exists and that reading may read it".to_owned()
        )
    );
}
