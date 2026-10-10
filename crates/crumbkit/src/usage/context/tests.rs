use super::Context;

/// The caller's TRACEPARENT in the tests, with flags `ff`.
const CALLER: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-ff";

/// The random bytes of a new trace id in the tests.
const NEW_TRACE: [u8; 16] = [0x11; 16];

/// The random bytes of the run's id in the tests.
const RUN: [u8; 8] = [0x22; 8];

/// Returns the context of a run whose caller set `traceparent` and
/// `tracestate`.
fn context(traceparent: Option<&str>, tracestate: Option<&str>) -> Context {
    Context::new(traceparent, tracestate.map(str::to_owned), NEW_TRACE, RUN).unwrap()
}

#[test]
fn a_valid_traceparent_gives_the_run_the_caller_s_trace_and_names_the_caller_as_its_parent() {
    let ids = context(Some(CALLER), None).ids;

    assert_eq!(ids.trace.as_str(), "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(ids.run.as_str(), "2222222222222222");
    assert_eq!(ids.parent.unwrap().as_str(), "00f067aa0ba902b7");
}

#[test]
fn a_malformed_or_missing_traceparent_starts_a_new_trace_with_no_parent() {
    let all_zero = "00-00000000000000000000000000000000-00f067aa0ba902b7-01";

    for traceparent in [None, Some("s3cr3t"), Some(all_zero)] {
        let ids = context(traceparent, Some("vendor=s3cr3t")).ids;

        assert_eq!(ids.trace.as_str(), "11111111111111111111111111111111");
        assert_eq!(ids.run.as_str(), "2222222222222222");
        assert_eq!(ids.parent, None);
    }
}

#[test]
fn random_bytes_that_are_all_zero_give_no_context() {
    assert!(Context::new(None, None, [0; 16], RUN).is_none());
    assert!(Context::new(Some(CALLER), None, NEW_TRACE, [0; 8]).is_none());
}

#[test]
fn a_child_s_traceparent_names_this_run_as_parent_and_keeps_the_trace_and_the_known_flags() {
    let env = context(Some(CALLER), None).child_env();

    assert_eq!(
        env,
        [(
            "TRACEPARENT",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-2222222222222222-03".to_owned()
        )]
    );
}

#[test]
fn a_new_trace_gives_its_children_the_sampled_and_random_flags() {
    let env = context(None, None).child_env();

    assert_eq!(
        env,
        [(
            "TRACEPARENT",
            "00-11111111111111111111111111111111-2222222222222222-03".to_owned()
        )]
    );
}

#[test]
fn tracestate_passes_to_a_child_unchanged_only_with_a_valid_traceparent() {
    let valid = context(Some(CALLER), Some("vendor=opaque,other=1")).child_env();
    let invalid = context(Some("s3cr3t"), Some("vendor=opaque")).child_env();

    assert_eq!(valid[1], ("TRACESTATE", "vendor=opaque,other=1".to_owned()));
    assert_eq!(invalid.len(), 1);
}
