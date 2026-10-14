//! Who ran the tool, and the session the run belongs to.

use std::io::{self, IsTerminal as _};
use std::path::Path;

use super::platform;

#[cfg(test)]
mod tests;

/// Who ran the tool, and the session the run belongs to.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Caller {
    /// The agent's id from is-ai-agent, such as `claude-code`; `person`
    /// when no agent is detected and standard input is a terminal; and
    /// `unknown` otherwise.
    pub(super) name: &'static str,
    /// The agent's session id, or the platform's fallback when the agent
    /// gives none or no agent is detected: the parent process id on Unix,
    /// which stays the same for a person typing in one shell, and the
    /// Windows Terminal session or the text `windows` on Windows. The usage
    /// log stores only its keyed hash.
    pub(super) session: String,
}

/// Returns the caller of this process, from the agent that is-ai-agent
/// finds, whether standard input is a terminal, and the platform's fallback
/// session. This module owns those environment, file and standard-input
/// reads (RUST-4).
pub(super) fn detect() -> Caller {
    #[expect(
        clippy::disallowed_methods,
        reason = "the caller module owns the environment reads that detect a coding agent"
    )]
    let env = |name: &str| std::env::var(name).ok();
    let agent = is_ai_agent::detect_with(env, |path| Path::new(path).exists());
    #[expect(
        clippy::disallowed_methods,
        reason = "the caller module asks whether standard input is a terminal, to tell a person from a program"
    )]
    let input = io::stdin();
    identify(agent, input.is_terminal(), platform::fallback_session)
}

/// Returns the caller from the detected `agent`, whether standard input is
/// a `terminal`, and the `fallback` session, as [`detect`] reads them.
fn identify(
    agent: Option<is_ai_agent::Agent>,
    terminal: bool,
    fallback: impl FnOnce() -> String,
) -> Caller {
    match agent {
        Some(agent) => Caller {
            name: agent.id.as_str(),
            session: agent.session_id.unwrap_or_else(fallback),
        },
        None => Caller {
            name: if terminal { "person" } else { "unknown" },
            session: fallback(),
        },
    }
}
