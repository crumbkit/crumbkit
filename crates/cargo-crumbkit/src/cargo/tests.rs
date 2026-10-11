//! Tests for building the binary with cargo.

use std::ffi::OsStr;
use std::path::PathBuf;

use crumbkit::{Failure as _, Spelling};

use super::{Built, build, built};

/// cargo's messages for a build of shelf: a library artifact, the binary's
/// artifact and the closing message.
const MESSAGES: &str = r#"{"reason":"compiler-artifact","manifest_path":"/work/crates/crumbkit/Cargo.toml","target":{"kind":["lib"],"name":"crumbkit"},"executable":null}
not JSON
{"reason":"compiler-artifact","manifest_path":"/work/examples/shelf/Cargo.toml","target":{"kind":["bin"],"name":"shelf"},"executable":"/work/target/debug/shelf"}
{"reason":"build-finished","success":true}
"#;

#[test]
fn the_binary_s_artifact_message_gives_its_executable_and_its_package_directory() {
    assert_eq!(
        built(MESSAGES.as_bytes(), "shelf"),
        Some(Built {
            executable: PathBuf::from("/work/target/debug/shelf"),
            package: PathBuf::from("/work/examples/shelf"),
        })
    );
}

#[test]
fn messages_without_an_executable_for_the_binary_give_none() {
    assert_eq!(built(MESSAGES.as_bytes(), "crumbkit"), None);
    assert_eq!(built(MESSAGES.as_bytes(), "atlas"), None);
}

#[test]
fn a_cargo_that_cannot_start_fails_with_cargo_unavailable() {
    let error = build(OsStr::new("/nonexistent/cargo"), "shelf").unwrap_err();

    assert_eq!(
        Spelling::COLONS.spell(error.code()),
        "crumbkit::weigh::cargo_unavailable"
    );
}

#[test]
fn a_binary_that_the_workspace_lacks_fails_with_build_failed() {
    let error = build(OsStr::new(env!("CARGO")), "no-such-binary").unwrap_err();

    assert_eq!(
        Spelling::COLONS.spell(error.code()),
        "crumbkit::weigh::build_failed"
    );
}
