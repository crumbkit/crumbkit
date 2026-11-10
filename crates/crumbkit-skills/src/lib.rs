// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`Skill`]: crate::Skill
//! [`SkillsInstall`]: crate::SkillsInstall
//! [`install`]: crate::install()
//! [`SKILLS_INSTALL`]: crate::SKILLS_INSTALL
//! [`InstallRow`]: crate::InstallRow
//! [`SkillsCheck`]: crate::SkillsCheck
//! [`check`]: crate::check()
//! [`SKILLS_CHECK`]: crate::SKILLS_CHECK
//! [`CheckRow`]: crate::CheckRow
//! [`print`]: crate::print()
//! [`SKILLS_PRINT`]: crate::SKILLS_PRINT
//! [`SkillText`]: crate::SkillText
//! [`cli.rs`]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/src/cli.rs
//! [`lib.rs`]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/src/lib.rs
//! [schemas]: https://github.com/crumbkit/crumbkit/tree/main/crates/crumbkit-skills/schemas
//! [ADR-0192]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0192-agent-skills.md
#![doc = include_str!("../README.md")]

mod check;
mod install;
mod print;
mod skill;
mod targets;
#[cfg(test)]
mod testing;

pub use self::check::{CheckRow, SKILLS_CHECK, SkillsCheck, check};
pub use self::install::{InstallRow, SKILLS_INSTALL, SkillsInstall, install};
pub use self::print::{SKILLS_PRINT, SkillText, print};
pub use self::skill::Skill;
