use super::*;
use crate::{Spelling, Tool};

#[test]
fn the_interrupted_failure_has_the_run_interrupted_code_and_advises_a_retry() {
    assert_eq!(
        Spelling::COLONS.spell(Interrupted.code()),
        "crumbkit::run::interrupted"
    );
    assert_eq!(Interrupted.recovery(), Recovery::Retry);
}

#[test]
fn the_interrupted_failure_exits_130_unless_the_tool_record_lists_another_exit() {
    use crate::tool::{Exit, Failures};

    const MAPPED: Tool = Tool::new().failures(Failures::new().exits(&[
        Exit::new(75, "The command was interrupted.").failures(&[codes::RUN_INTERRUPTED]),
    ]));
    assert_eq!(Tool::new().failure_exit(codes::RUN_INTERRUPTED), 130);
    assert_eq!(MAPPED.failure_exit(codes::RUN_INTERRUPTED), 75);
}

#[test]
fn a_sigterm_always_ends_the_process_and_a_sigint_outside_a_window_does_too() {
    assert_eq!(decide(signal_hook::consts::SIGTERM), Decision::End);
    assert_eq!(decide(SIGINT), Decision::End);
}

#[test]
fn clean_ups_are_kept_once_each() {
    fn one() {}
    clean_up(one);
    clean_up(one);

    let kept = CLEAN_UPS.lock().unwrap();
    let count = kept
        .iter()
        .filter(|kept| std::ptr::fn_addr_eq(**kept, one as fn()))
        .count();
    assert_eq!(count, 1);
}
