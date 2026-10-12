//! Who ran the tool, and the session the run belongs to.

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

/// Returns the caller of this process, from the agent that
/// [`crate::agent::detect`] finds, its standard input and its parent
/// process.
pub(super) fn detect() -> Caller {
    identify(
        crate::agent::detect(),
        crate::sink::stdin_is_terminal(),
        std::os::unix::process::parent_id(),
    )
}

/// Returns the caller from the detected `agent`, whether standard input is
/// a `terminal`, and the `parent` process id, as [`detect`] reads them.
fn identify(agent: Option<is_ai_agent::Agent>, terminal: bool, parent: u32) -> Caller {
    match agent {
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
