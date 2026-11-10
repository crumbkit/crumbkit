//! `cargo crumbkit skill`, the commands for a tool's agent skill: the
//! SKILL.md file that tells an agent how to use the tool.

mod check;
mod tree;

pub(crate) use check::{SKILL_CHECK, check};
