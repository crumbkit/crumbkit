//! Agent detection: whether a coding agent runs the tool, which decides
//! whether a person can be asked a question (ADR-0043) and which caller the
//! usage log names (ADR-0029).

use std::path::Path;

#[cfg(test)]
mod tests;

/// Returns the coding agent that runs this process, read from its
/// environment and from the files that mark an agent, or `None` when no
/// agent is detected. This module owns those reads (RUST-4).
pub(crate) fn detect() -> Option<is_ai_agent::Agent> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the agent module owns the environment reads that detect a coding agent"
    )]
    let env = |name: &str| std::env::var(name).ok();
    is_ai_agent::detect_with(env, |path| Path::new(path).exists())
}

/// Reports whether a person can answer a question: standard input and
/// standard error are both terminals, which `terminals` reports, and no
/// coding agent runs the tool, which `agent` reports. `agent` runs only when
/// both are terminals, so a run without a terminal reads no environment.
pub(crate) fn person_can_answer(terminals: bool, agent: impl FnOnce() -> bool) -> bool {
    terminals && !agent()
}
