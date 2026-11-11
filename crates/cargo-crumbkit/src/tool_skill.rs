//! `cargo crumbkit skill`, the commands that draft and check a tool's agent
//! skill: the SKILL.md file that tells an agent how to use the tool.

mod check;
mod draft;
mod tree;

pub(crate) use check::{TOOL_SKILL_CHECK, check};
pub(crate) use draft::{TOOL_SKILL_DRAFT, draft};
