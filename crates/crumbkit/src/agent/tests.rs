use std::cell::Cell;

use super::person_can_answer;

#[test]
fn a_person_can_answer_at_two_terminals_without_an_agent() {
    assert!(person_can_answer(true, || false));
}

#[test]
fn nobody_can_answer_when_an_agent_runs_the_tool_at_two_terminals() {
    assert!(!person_can_answer(true, || true));
}

#[test]
fn without_terminals_nobody_can_answer_and_the_agent_check_never_runs() {
    let checked = Cell::new(false);

    let answer = person_can_answer(false, || {
        checked.set(true);
        false
    });

    assert!(!answer);
    assert!(!checked.get());
}
