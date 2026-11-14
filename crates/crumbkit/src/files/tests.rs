use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{display, home_in};

/// Returns an absolute home directory. On Windows an absolute path needs a
/// drive, so `/home/ada` would not count.
fn absolute_home() -> PathBuf {
    let root = if cfg!(windows) { r"C:\" } else { "/" };
    Path::new(root).join("home").join("ada")
}

#[test]
fn an_absolute_home_is_the_home_directory() {
    let home = absolute_home();

    let found = home_in(|name| (name == "HOME").then(|| home.clone().into_os_string()));

    assert_eq!(found, Some(home));
}

#[test]
fn a_relative_empty_or_unset_home_is_no_home_directory() {
    assert_eq!(home_in(|_| Some(OsString::from("ada"))), None);
    assert_eq!(home_in(|_| Some(OsString::new())), None);
    assert_eq!(home_in(|_| None), None);
}

#[cfg(windows)]
#[test]
fn userprofile_names_the_home_directory_on_windows_when_home_is_unset() {
    let home = home_in(|name| (name == "USERPROFILE").then(|| OsString::from(r"C:\Users\ada")));

    assert_eq!(home, Some(r"C:\Users\ada".into()));
}

#[test]
fn a_verbatim_drive_path_displays_without_its_prefix() {
    assert_eq!(display(Path::new(r"\\?\C:\Users\ada")), r"C:\Users\ada");
}

#[test]
fn a_verbatim_unc_path_and_a_unix_path_display_unchanged() {
    assert_eq!(
        display(Path::new(r"\\?\UNC\server\share")),
        r"\\?\UNC\server\share"
    );
    assert_eq!(display(Path::new("/home/ada")), "/home/ada");
}
