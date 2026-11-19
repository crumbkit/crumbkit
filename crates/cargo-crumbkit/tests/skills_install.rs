//! End-to-end test of `cargo crumbkit skills install`, which installs cargo
//! crumbkit's own skill with every file beside its `SKILL.md`.

use std::fs;
use std::process::Command;

/// Each file of cargo crumbkit's skill folder, as the binary should install
/// it.
const FILES: [(&str, &str); 5] = [
    ("SKILL.md", include_str!("../skill/SKILL.md")),
    ("AFFORDANCES.md", include_str!("../skill/AFFORDANCES.md")),
    (
        "audit-report-v1.json",
        include_str!("../skill/audit-report-v1.json"),
    ),
    ("GUIDES.md", include_str!("../skill/GUIDES.md")),
    (
        "guide-report-v1.json",
        include_str!("../skill/guide-report-v1.json"),
    ),
];

/// Returns the text of a `SKILL.md` after its frontmatter.
fn body(skill: &str) -> Option<&str> {
    skill
        .strip_prefix("---\n")?
        .split_once("\n---\n")
        .map(|(_, body)| body)
}

#[test]
fn skills_install_writes_skill_md_and_all_four_files_beside_it() {
    let home = tempfile::tempdir().unwrap();
    let skills = home.path().join(".claude").join("skills");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test creates the agent skills directory that install writes into"
    )]
    fs::create_dir_all(&skills).unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs the built binary as cargo runs a subcommand"
    )]
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-crumbkit"))
        .args(["crumbkit", "skills", "install"])
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("CODEX_HOME", home.path().join(".codex"))
        .env("XDG_CONFIG_HOME", home.path().join("config"))
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_STATE_HOME", home.path().join("state"))
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let installed = skills.join("cargo-crumbkit");
    let mut names: Vec<String> = fs::read_dir(&installed)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort_unstable();
    let mut expected: Vec<&str> = FILES.iter().map(|(name, _)| *name).collect();
    expected.sort_unstable();
    assert_eq!(names, expected);
    for (name, text) in FILES {
        let written = fs::read_to_string(installed.join(name)).unwrap();
        if name == "SKILL.md" {
            // install adds its own metadata to the frontmatter, so only the
            // body after the frontmatter matches the shipped text.
            assert!(body(text).is_some(), "SKILL.md has no frontmatter");
            assert_eq!(body(&written), body(text));
        } else {
            assert_eq!(written, text, "{name}");
        }
    }
}
