//! Who ran the tool, and the session the run belongs to.

use std::path::Path;

#[cfg(test)]
mod tests;

/// Who ran the tool, and the session the run belongs to.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Caller {
    /// The agent's id from is-ai-agent, such as `claude-code`; `person`
    /// when no agent is detected and standard input is a terminal; and
    /// `unknown` otherwise.
    pub(super) name: &'static str,
    /// The agent's session id, or the parent process id when the agent gives
    /// none or no agent is detected. A person typing in one shell keeps one
    /// parent process id. The usage log stores only its keyed hash.
    pub(super) session: String,
}

/// Returns the caller of this process, from its environment, the files that
/// mark an agent, its standard input and its parent process.
pub(super) fn detect() -> Caller {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its environment reads, which tell which agent ran the tool"
    )]
    let env = |name: &str| std::env::var(name).ok();
    identify(
        env,
        |path| Path::new(path).exists(),
        crate::sink::stdin_is_terminal(),
        std::os::unix::process::parent_id(),
    )
}

/// Returns the caller from an environment lookup `env`, a file check
/// `file_exists`, whether standard input is a `terminal`, and the `parent`
/// process id, as [`detect`] reads them.
fn identify(
    env: impl Fn(&str) -> Option<String>,
    file_exists: impl Fn(&str) -> bool,
    terminal: bool,
    parent: u32,
) -> Caller {
    match is_ai_agent::detect_with(env, file_exists) {
        Some(agent) => Caller {
            name: agent.id.as_str(),
            session: agent.session_id.unwrap_or_else(|| parent.to_string()),
        },
        None => Caller {
            name: if terminal { "person" } else { "unknown" },
            session: parent.to_string(),
        },
    }
}
