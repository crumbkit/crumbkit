use std::ffi::OsString;
use std::path::PathBuf;

use super::{session_from, state_home_from};

#[test]
fn an_absolute_xdg_state_home_is_the_state_directory() {
    let state = state_home_from(Some(OsString::from(r"C:\state")), || None);

    assert_eq!(state, Some(PathBuf::from(r"C:\state")));
}

#[test]
fn a_relative_xdg_state_home_is_ignored_for_the_local_application_data_directory() {
    let local = || Some(PathBuf::from(r"C:\Users\me\AppData\Local"));

    assert_eq!(
        state_home_from(Some(OsString::from("state")), local),
        local()
    );
    assert_eq!(state_home_from(None, local), local());
}

#[test]
fn the_session_is_the_windows_terminal_session_when_one_is_set() {
    assert_eq!(session_from(Some("tab-1".to_owned())), "tab-1");
}

#[test]
fn the_session_is_windows_outside_windows_terminal() {
    assert_eq!(session_from(None), "windows");
    assert_eq!(session_from(Some(String::new())), "windows");
}
