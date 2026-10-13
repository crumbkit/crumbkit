use super::{Context, tracestate};

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
fn tracestate_passes_to_a_child_unchanged_with_a_valid_traceparent() {
    let env = context(Some(CALLER), Some("vendor=opaque,other=1")).child_env();

    assert_eq!(env[1], ("TRACESTATE", "vendor=opaque,other=1".to_owned()));
}

#[test]
fn a_new_trace_gives_a_child_an_empty_tracestate_when_the_caller_set_one() {
    let set = context(Some("s3cr3t"), Some("vendor=opaque")).child_env();
    let unset = context(Some("s3cr3t"), None).child_env();

    assert_eq!(set[1], ("TRACESTATE", String::new()));
    assert_eq!(set.len(), 2);
    assert_eq!(unset.len(), 1);
}

/// Returns `v` followed by a value that is not UTF-8: the byte 0xFF on
/// Unix, and a lone surrogate on Windows.
fn not_utf_8() -> std::ffi::OsString {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(vec![0x76, 0xff])
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStringExt as _;
        std::ffi::OsString::from_wide(&[0x76, 0xD800])
    }
}

#[test]
fn a_tracestate_that_is_not_utf_8_counts_as_empty() {
    let state = tracestate(Some(not_utf_8()));

    assert_eq!(state, Some(String::new()));
    assert_eq!(tracestate(None), None);
}

#[cfg(feature = "complete")]
#[test]
fn the_usage_log_skips_a_run_that_carries_the_completion_variable_that_the_tool_declared() {
    let declared = crate::complete_with("SHELF_COMPLETE", || {
        panic!("an ordinary run builds no command for completion")
    });
    let shell = || Some(std::ffi::OsString::from("fish"));

    assert!(declared.is_none());
    assert!(super::completing_in(|name| (name == "SHELF_COMPLETE")
        .then(shell)
        .flatten()));
    assert!(!super::completing_in(|name| (name == "COMPLETE")
        .then(shell)
        .flatten()));
}
