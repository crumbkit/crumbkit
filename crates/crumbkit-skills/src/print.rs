//! The `skills print` command, which prints one of a tool's skills
//! (ADR-0192, ADR-0201).
//!
//! A tool mounts the command with [`SkillsCommand`](crate::SkillsCommand),
//! whose `Print` variant has no arguments, and one call to
//! [`run`](crate::run), which passes the result of [`print`] for the first
//! skill to [`finish`](crumbkit::finish) with the document
//! [`SKILLS_PRINT`]:
//!
//! ```ignore
//! Resource::Skills { command } => crumbkit_skills::run(run, &[SKILL], &command),
//! ```
//!
//! A tool with several skills that print by name gives its own `print`
//! subcommand an argument that chooses the skill, and calls [`print`]
//! itself.

use std::fmt;

use crumbkit::{Document, Prose};
use schemars::JsonSchema;
use serde::Serialize;

use crate::skill::Skill;

#[cfg(test)]
mod tests;

/// The document that `skills print` prints: one of the tool's skills. Its schema is
/// `crates/crumbkit-skills/schemas/skills-print-v1.json`.
pub const SKILLS_PRINT: Document<Prose<SkillText>> = Document::new(
    "crumbkit",
    "skills-print",
    1,
    "One agent skill of the tool: the tool's name and version, the skill's name, the text of its SKILL.md and the names of its other files.",
)
.always_print_first()
.built_in();

/// A tool's skill as `skills print` prints it. The text output is the
/// skill's text, and the JSON document holds the tool's name, the skill's
/// name, the tool's version and the names of the skill's other files
/// beside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SkillText {
    /// The tool's name, such as `shelf`.
    tool: String,
    /// The skill's name, which names the skill's folder, such as `shelf`.
    skill: String,
    /// The tool's version, such as `0.3.0`.
    version: String,
    /// The text of the skill's SKILL.md, as the tool embeds it, without the metadata lines that install adds.
    text: String,
    /// The names of the skill's other files, which install writes beside SKILL.md, in the order the tool gives them.
    files: Vec<String>,
}

impl fmt::Display for SkillText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// Returns `skill` for the tool to pass to [`finish`](crumbkit::finish)
/// with [`SKILLS_PRINT`].
///
/// The text is the skill as the tool embeds it, without the `metadata:`
/// lines that [`install`](crate::install) adds, so it reads the same in
/// every build of one version. The document always prints the text whole,
/// even when it passes the tool's output budget, because a cut skill is of
/// no use to an agent. The skill's other files, which [`Skill::files`]
/// names, are listed by name under `files`, and the text output names them
/// in a note on standard error.
///
/// It takes one skill. A tool with several skills chooses which one to
/// pass, such as by an argument of its own `skills print` command.
#[must_use]
pub fn print(skill: &Skill) -> Prose<SkillText> {
    let files: Vec<String> = skill
        .get_files()
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    let note = match files.as_slice() {
        [] => None,
        [one] => Some(one.clone()),
        [rest @ .., last] => Some(format!("{} and {last}", rest.join(", "))),
    };
    let prose = Prose::new(SkillText {
        tool: skill.tool().to_owned(),
        skill: skill.name().to_owned(),
        version: skill.version().to_owned(),
        text: skill.text().to_owned(),
        files,
    });
    match note {
        Some(names) => prose.note(format!("The skill also holds {names} beside SKILL.md.")),
        None => prose,
    }
}
