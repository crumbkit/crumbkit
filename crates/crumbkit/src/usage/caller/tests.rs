use super::{Caller, identify};

/// Returns the agent that is-ai-agent detects in an environment that holds
/// only `pairs`, with no marker files.
fn agent(pairs: &'static [(&'static str, &'static str)]) -> Option<is_ai_agent::Agent> {
    is_ai_agent::detect_with(env(pairs), |_| false)
}

/// Returns an environment that holds only `pairs`.
fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    }
}

#[test]
fn an_agent_with_a_session_id_is_the_caller_and_its_session_id_is_the_session() {
    let caller = identify(
        agent(&[("CLAUDECODE", "1"), ("CLAUDE_CODE_SESSION_ID", "s-1")]),
        true,
        4242,
    );

    assert_eq!(
        caller,
        Caller {
            name: "claude-code",
            session: "s-1".to_owned()
        }
    );
}

#[test]
fn an_agent_without_a_session_id_has_the_parent_process_id_as_its_session() {
    let caller = identify(agent(&[("CLAUDECODE", "1")]), false, 4242);

    assert_eq!(
        caller,
        Caller {
            name: "claude-code",
            session: "4242".to_owned()
        }
    );
}

#[test]
fn without_an_agent_a_terminal_on_standard_input_means_a_person() {
    let caller = identify(agent(&[]), true, 4242);

    assert_eq!(
        caller,
        Caller {
            name: "person",
            session: "4242".to_owned()
        }
    );
}

#[test]
fn without_an_agent_or_a_terminal_the_caller_is_unknown() {
    let caller = identify(agent(&[]), false, 4242);

    assert_eq!(caller.name, "unknown");
}
