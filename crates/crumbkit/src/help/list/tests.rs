use std::borrow::Cow;

use clap::Command;

use super::description;

#[test]
fn a_description_without_visible_aliases_borrows_the_command_s_about() {
    let command = Command::new("list").about("List the moons");
    assert!(matches!(description(&command), Some(Cow::Borrowed(_))));
}

#[test]
fn a_description_with_a_visible_alias_owns_its_text() {
    let command = Command::new("list")
        .about("List the moons")
        .visible_alias("ls");
    assert!(matches!(description(&command), Some(Cow::Owned(_))));
}
