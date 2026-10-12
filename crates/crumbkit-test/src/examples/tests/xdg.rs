//! Tests for the XDG fixture that the example check copies into each run's
//! fresh XDG directories.

use std::fs;

use crumbkit::help::{Example, Guide, Leaf};
use snapbox::assert::Action;

use super::{examples, run};
use crate::temp::TempDir;
use crate::testing::panic_message;

/// The guide of two examples: the first prints the plugin file of the
/// fixture and the path of its data directory, then deletes the file, and
/// the second prints the file again.
const PLUGIN: Guide = Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[
    Example::new(
        &[r#"cat "$XDG_DATA_HOME/tool/plugin"; echo "$XDG_DATA_HOME"; rm "$XDG_DATA_HOME/tool/plugin""#],
        "Read and remove the plugin",
    ),
    Example::new(&[r#"cat "$XDG_DATA_HOME/tool/plugin""#], "Read the plugin"),
])]);

/// Returns a fixture whose `data` directory holds `tool/plugin`.
fn fixture(label: &str) -> TempDir {
    let fixture = TempDir::new(label).unwrap();
    fixture.create_dir("data/tool").unwrap();
    fixture.write("data/tool/plugin", "plugin\n");
    fixture
}

#[test]
fn each_example_run_starts_from_a_fresh_copy_of_the_xdg_fixture_and_its_paths_are_redacted() {
    let dir = TempDir::new("examples-xdg").unwrap();
    let fixture = fixture("examples-xdg-fixture");
    let path = fixture.path().display().to_string();

    run(&PLUGIN, &examples(&dir).xdg(&path), Action::Overwrite);

    let first = fs::read_to_string(dir.path().join(
        "sh-c-cat-XDG_DATA_HOME-tool-plugin-echo-XDG_DATA_HOME-rm-XDG_DATA_HOME-tool-plugin.txt",
    ))
    .unwrap();
    let second =
        fs::read_to_string(dir.path().join("sh-c-cat-XDG_DATA_HOME-tool-plugin.txt")).unwrap();
    assert_eq!(first, "plugin\n[XDG_DATA_HOME]\n");
    assert_eq!(second, "plugin\n");
    assert!(fixture.path().join("data/tool/plugin").is_file());
}

#[test]
fn a_missing_xdg_fixture_or_an_entry_other_than_the_four_directories_fails_the_check_before_any_run()
 {
    let dir = TempDir::new("examples-xdg-bad").unwrap();
    let fixture = TempDir::new("examples-xdg-bad-fixture").unwrap();
    fixture.create_dir("configs").unwrap();
    let path = fixture.path().display().to_string();
    let missing = fixture.path().join("absent").display().to_string();

    let wrong = panic_message(|| run(&PLUGIN, &examples(&dir).xdg(&path), Action::Overwrite));
    let absent = panic_message(|| run(&PLUGIN, &examples(&dir).xdg(&missing), Action::Overwrite));

    assert_eq!(
        wrong.unwrap(),
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             the XDG fixture `{path}` holds `configs`, and a fixture holds only the directories config, data, state and cache"
        )
    );
    assert_eq!(
        absent.unwrap(),
        format!(
            "the command tree breaks crumbkit's example rules:\n\
             the XDG fixture `{missing}` is not a directory"
        )
    );
    assert_eq!(dir.names(), Vec::<String>::new());
}

#[test]
fn a_symbolic_link_in_the_xdg_fixture_fails_the_run() {
    let dir = TempDir::new("examples-xdg-link").unwrap();
    let fixture = fixture("examples-xdg-link-fixture");
    let link = fixture.path().join("data/tool/link");
    std::os::unix::fs::symlink("plugin", &link).unwrap();
    let path = fixture.path().display().to_string();

    let message =
        panic_message(|| run(&PLUGIN, &examples(&dir).xdg(&path), Action::Overwrite)).unwrap();

    assert!(
        message.contains(&format!(
            "the XDG fixture `{path}` cannot be copied: `{}` is a symbolic link, which the check does not copy",
            link.display()
        )),
        "{message}"
    );
}
