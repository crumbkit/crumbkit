use std::time::Duration;

use super::Escalation;
use crate::Signal;

#[test]
fn the_default_escalation_has_no_steps_so_a_stop_kills_at_once() {
    assert_eq!(Escalation::default().steps().count(), 0);
}

#[test]
fn each_step_keeps_its_signal_and_its_grace_in_the_order_given() {
    let escalation = Escalation::default()
        .step(Signal::Hangup, Duration::from_millis(5))
        .step(Signal::Terminate, Duration::from_millis(7));

    let steps: Vec<_> = escalation.steps().collect();

    assert_eq!(
        steps,
        [
            (Signal::Hangup, Duration::from_millis(5)),
            (Signal::Terminate, Duration::from_millis(7)),
        ]
    );
}
