use std::path::PathBuf;

use super::file_in;

#[test]
fn an_absolute_xdg_config_home_holds_the_tool_s_aliases_file() {
    assert_eq!(
        file_in("shelf", Some("/xdg/config".into())),
        Some(PathBuf::from("/xdg/config/shelf/aliases.toml")),
    );
}

#[test]
fn a_relative_xdg_config_home_is_ignored() {
    assert_ne!(
        file_in("shelf", Some("config".into())),
        Some(PathBuf::from("config/shelf/aliases.toml")),
    );
}
