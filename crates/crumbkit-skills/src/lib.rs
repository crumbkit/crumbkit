// The crate docs are the README, so its example runs as a doctest. The README
// links to files in the repository, which work on GitHub. The definitions below
// come first, so rustdoc uses them for the same labels (ADR-0013).
//! [`Skill`]: crate::Skill
//! [`Skill::files`]: crate::Skill::files
//! [`SkillsInstall`]: crate::SkillsInstall
//! [`install`]: crate::install()
//! [`SKILLS_INSTALL`]: crate::SKILLS_INSTALL
//! [`InstallRow`]: crate::InstallRow
//! [`SkillsCheck`]: crate::SkillsCheck
//! [`check`]: crate::check()
//! [`SKILLS_CHECK`]: crate::SKILLS_CHECK
//! [`CheckRow`]: crate::CheckRow
//! [`SkillsAction`]: crate::SkillsAction
//! [`skills_command!`]: crate::skills_command
//! [`run`]: crate::run()
//! [`run_to`]: crate::run_to()
//! [`print`]: crate::print()
//! [`SKILLS_PRINT`]: crate::SKILLS_PRINT
//! [`SkillText`]: crate::SkillText
//! [`cli.rs`]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/src/cli.rs
//! [`lib.rs`]: https://github.com/crumbkit/crumbkit/blob/main/examples/shelf/src/lib.rs
//! [schemas]: https://github.com/crumbkit/crumbkit/tree/main/crates/crumbkit-skills/schemas
//! [ADR-0192]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0192-agent-skills.md
//! [ADR-0199]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/skill/0199-skill-files.md
//! [ADR-0201]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/skill/0201-several-skills.md
//! [ADR-0231]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/skill/0231-skills-help-names-the-tool.md
//! [ADR-0216]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/help/skill/0216-a-tool-mounts-its-skills-commands-from-crumbkit-skills.md
#![doc = include_str!("../README.md")]

mod check;
mod install;
mod mount;
mod print;
mod skill;
mod targets;
#[cfg(test)]
mod testing;

pub use self::check::{CheckRow, SKILLS_CHECK, SkillsCheck, check};
pub use self::install::{InstallRow, SKILLS_INSTALL, SkillsInstall, install};
pub use self::mount::{SkillsAction, SkillsPrint, run, run_to};
pub use self::print::{SKILLS_PRINT, SkillText, print};
pub use self::skill::Skill;

/// Items that [`skills_command!`] expands to. They are not a stable interface.
#[doc(hidden)]
pub mod __private {
    pub use crumbkit::help::{Example, Leaf};

    pub use crate::mount::{check_leaf, install_leaf, print_leaf};
}
