//! Tests for writing the generated sections of a skill file.

use super::{description, jobs, most_used, skeleton};
use crate::skill::draft::usage::Ranked;
use crate::skill::tree::Tree;

/// A `shelf/commands-list/v1` document of shelf's root with goal headings,
/// `books`, `books list` and `books show`.
const DOCUMENT: &str = r#"{"schema":"shelf/commands-list/v1","data":[
{"path":"shelf","about":"Browse a catalogue: books | more","goals":[
{"heading":"Browse the catalogue","commands":["books"]}],"arguments":[
{"name":"--json","short":null,"value_names":[],"many":false,"possible_values":[],"global":true}]},
{"path":"shelf books","about":"Work with the books","arguments":[]},
{"path":"shelf books list","about":"List the books","arguments":[
{"name":"--limit","short":null,"value_names":["n"],"many":false,"possible_values":[],"global":false}]},
{"path":"shelf books show","about":null,"arguments":[]}
]}"#;

/// Returns the tree of [`DOCUMENT`].
fn tree() -> Tree {
    Tree::read(DOCUMENT.as_bytes()).unwrap()
}

#[test]
fn the_description_is_the_root_s_about_text_as_a_quoted_yaml_value() {
    assert_eq!(
        description(&tree()),
        "description: \"Browse a catalogue: books | more\"\n"
    );
}

#[test]
fn the_job_table_lists_each_goal_heading_with_its_commands_and_their_about_text() {
    assert_eq!(
        jobs(&tree()),
        "## Jobs\n\
         \n\
         | Job | Command | What it does |\n\
         | --- | --- | --- |\n\
         | Browse the catalogue | `shelf books` | Work with the books |\n"
    );
}

#[test]
fn the_most_used_section_names_each_command_with_its_common_flags_and_about_text() {
    let ranked = [
        Ranked {
            command: "books list".to_owned(),
            runs: 3,
            wasted: 1,
            flags: vec!["--limit".to_owned(), "--json".to_owned()],
        },
        Ranked {
            command: "books show".to_owned(),
            runs: 1,
            wasted: 0,
            flags: Vec::new(),
        },
    ];

    let text = most_used(&tree(), &ranked);

    assert!(
        text.ends_with(
            "- `shelf books list [--limit N] [--json]`: List the books\n\
             - `shelf books show`\n"
        ),
        "{text}"
    );
}

#[test]
fn a_skeleton_holds_frontmatter_a_hand_written_area_and_every_section() {
    let text = skeleton(&tree(), "description: x\n", "jobs\n", "usage\n");

    assert!(text.starts_with(
        "---\nname: shelf\n# crumbkit:generated description\ndescription: x\n# /crumbkit:generated description\n---\n"
    ));
    assert!(
        text.contains(
            "<!-- crumbkit:generated jobs -->\njobs\n<!-- /crumbkit:generated jobs -->\n"
        )
    );
    assert!(text.ends_with(
        "<!-- crumbkit:generated usage -->\nusage\n<!-- /crumbkit:generated usage -->\n"
    ));
}
