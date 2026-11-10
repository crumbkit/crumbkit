//! Who ran the tool, the session the run belongs to, and the program that
//! marked the run.

use std::env::VarError;
use std::io::{self, IsTerminal as _, Write as _};
use std::path::Path;

use super::platform;
use super::program::{Program, RULE};
use crumbkit::ToolName;

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

/// Returns the program that marked a run of the tool named `tool`, from the
/// tool's `<TOOL>_PROGRAM` variable, such as `SHELF_PROGRAM`, or `None` when
/// the variable is unset, empty or refused.
///
/// The recorder calls it once, when `parse` gives it the tool's name, so a
/// run that never reaches `parse` reads no variable. A variable that holds
/// a value other than a program name prints a one-line warning on standard
/// error then, before the tool prints anything, and never on the thread
/// that answers a signal (ADR-0197). This module owns that environment read
/// and that warning (RUST-4).
pub(super) fn program(tool: ToolName) -> Option<Program> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the caller module owns the environment read of the program that marked the run"
    )]
    let mark = marked(tool, |name| std::env::var(name));
    if let Some(warning) = mark.warning(tool) {
        #[expect(
            clippy::disallowed_methods,
            reason = "the caller module warns on standard error that it ignored the program variable"
        )]
        let mut error = io::stderr();
        // A warning that cannot be written is dropped, as every other
        // usage-log failure is (ADR-0029).
        let _ = error.write_all(warning.as_bytes());
    }
    mark.program()
}

/// What the tool's `<TOOL>_PROGRAM` variable says about a run.
#[derive(Debug, PartialEq, Eq)]
enum Mark {
    /// The variable is unset or empty.
    Unmarked,
    /// The variable holds a program name.
    Program(Program),
    /// The variable holds a value that is not UTF-8 or not a program name.
    Refused,
}

impl Mark {
    /// Returns the program that marked the run, or `None` for a run that
    /// is unmarked or whose mark was refused.
    fn program(self) -> Option<Program> {
        match self {
            Self::Program(program) => Some(program),
            Self::Unmarked | Self::Refused => None,
        }
    }

    /// Returns the line that a run of the tool named `tool` prints on
    /// standard error when its mark was refused, or `None` otherwise. The
    /// line names the variable and the rule, and never the value, which may
    /// hold anything (ADR-0197).
    fn warning(&self, tool: ToolName) -> Option<String> {
        matches!(self, Self::Refused).then(|| {
            format!(
                "warning: {} was ignored, because {RULE}; the run is recorded without a program\n",
                tool.variable("PROGRAM")
            )
        })
    }
}

/// Returns the mark of a run of the tool named `tool`, from the tool's
/// `<TOOL>_PROGRAM` variable as `env` reads it.
fn marked(tool: ToolName, env: impl FnOnce(&str) -> Result<String, VarError>) -> Mark {
    match env(&tool.variable("PROGRAM")) {
        Err(VarError::NotPresent) => Mark::Unmarked,
        Ok(text) if text.is_empty() => Mark::Unmarked,
        Ok(text) => Program::new(&text).map_or(Mark::Refused, Mark::Program),
        Err(VarError::NotUnicode(_)) => Mark::Refused,
    }
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
