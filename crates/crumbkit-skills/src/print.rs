//! The `skills print` command, which prints a tool's skill (ADR-0192).
//!
//! The tool owns the command. It adds a `print` subcommand without
//! arguments to its `skills` group, and its handler passes the result of
//! [`print`] to [`finish`](crumbkit::finish) with the document
//! [`SKILLS_PRINT`]:
//!
//! ```ignore
//! SkillsCommand::Print => {
//!     let result = Ok::<_, crumbkit::Error>(crumbkit_skills::print(&SKILL));
//!     crumbkit::finish(run, crumbkit_skills::SKILLS_PRINT, result)
//! }
//! ```

use std::fmt;

use crumbkit::{Document, Prose};
use schemars::JsonSchema;
use serde::Serialize;

use crate::skill::Skill;

#[cfg(test)]
mod tests;

/// The document that `skills print` prints: the tool's skill. Its schema is
/// `crates/crumbkit-skills/schemas/skills-print-v1.json`.
pub const SKILLS_PRINT: Document<Prose<SkillText>> = Document::new(
    "crumbkit",
    "skills-print",
    1,
    "The tool's agent skill: its name, its version, the text of its SKILL.md and the names of its other files.",
)
.always_print_first()
.built_in();

/// A tool's skill as `skills print` prints it. The text output is the
/// skill's text, and the JSON document holds the tool's name, its version
/// and the names of the skill's other files beside it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct SkillText {
    /// The tool's name, which names the skill's folder, such as `shelf`.
    tool: String,
    /// The tool's version, such as `0.3.0`.
    version: String,
    /// The text of the tool's SKILL.md, as the tool embeds it, without the metadata lines that install adds.
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
        tool: skill.name().to_owned(),
        version: skill.version().to_owned(),
        text: skill.text().to_owned(),
        files,
    });
    match note {
        Some(names) => prose.note(format!("The skill also holds {names} beside SKILL.md.")),
        None => prose,
    }
}
