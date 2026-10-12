use std::path::{Path, PathBuf};

use super::file_in;

/// An absolute config directory on this platform.
#[cfg(unix)]
const ABSOLUTE_CONFIG: &str = "/xdg/config";
/// An absolute config directory on this platform.
#[cfg(windows)]
const ABSOLUTE_CONFIG: &str = r"C:\xdg\config";

#[test]
fn an_absolute_xdg_config_home_holds_the_tool_s_aliases_file() {
    assert_eq!(
        file_in("shelf", Some(ABSOLUTE_CONFIG.into())),
        Some(
            Path::new(ABSOLUTE_CONFIG)
                .join("shelf")
                .join("aliases.toml")
        ),
    );
}

#[test]
fn a_relative_xdg_config_home_is_ignored() {
    assert_ne!(
        file_in("shelf", Some("config".into())),
        Some(PathBuf::from("config/shelf/aliases.toml")),
    );
}
