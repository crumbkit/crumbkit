//! Tests for the location of the aliases file.

use std::ffi::OsString;
use std::path::PathBuf;

use super::locate;

/// Returns `text` as the value of a set environment variable.
fn var(text: &str) -> Option<OsString> {
    text.parse().ok()
}

#[test]
fn the_file_is_in_the_tool_s_directory_under_xdg_config_home() {
    let file = locate("shelf", var("/xdg/config"), var("/home/ada"));

    assert_eq!(file, Some(PathBuf::from("/xdg/config/shelf/aliases.toml")));
}

#[test]
fn without_xdg_config_home_the_file_is_under_dot_config_in_home() {
    let unset = locate("shelf", None, var("/home/ada"));
    let empty = locate("shelf", var(""), var("/home/ada"));

    let expected = Some(PathBuf::from("/home/ada/.config/shelf/aliases.toml"));
    assert_eq!(unset, expected);
    assert_eq!(empty, expected);
}

#[test]
fn a_relative_xdg_config_home_is_ignored_as_the_xdg_specification_says() {
    let file = locate("shelf", var("config"), var("/home/ada"));

    assert_eq!(
        file,
        Some(PathBuf::from("/home/ada/.config/shelf/aliases.toml"))
    );
}

#[test]
fn without_an_absolute_xdg_config_home_or_home_the_tool_has_no_aliases_file() {
    assert_eq!(locate("shelf", None, None), None);
    assert_eq!(locate("shelf", var("config"), var("")), None);
    assert_eq!(locate("shelf", None, var("home/ada")), None);
}
