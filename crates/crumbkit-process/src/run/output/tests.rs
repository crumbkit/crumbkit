use super::{Capture, Keeper, OverLimit, SLACK};

/// Returns what `capture` kept of `chunks`, and whether any chunk asked to
/// stop the child.
fn keep(capture: Capture, chunks: &[&[u8]]) -> (Vec<u8>, bool, bool) {
    let mut keeper = Keeper::new(capture);
    let stops = chunks
        .iter()
        .map(|chunk| keeper.add(chunk))
        .filter(|stop| *stop)
        .count();
    assert!(stops <= 1, "{stops} chunks asked to stop the child");
    let kept = keeper.finish();
    (kept.bytes().to_vec(), kept.over_limit(), stops == 1)
}

#[test]
fn a_discarded_stream_keeps_nothing_and_is_never_over_its_limit() {
    assert_eq!(
        keep(Capture::Discard, &[b"anything"]),
        (Vec::new(), false, false)
    );
}

#[test]
fn a_whole_stream_up_to_its_limit_is_kept_whole() {
    let whole = Capture::Whole {
        limit: 4,
        over: OverLimit::Stop,
    };

    assert_eq!(
        keep(whole, &[b"ab", b"cd"]),
        (b"abcd".to_vec(), false, false)
    );
}

#[test]
fn a_whole_stream_past_its_limit_keeps_its_first_bytes_and_asks_once_to_stop() {
    let whole = Capture::Whole {
        limit: 4,
        over: OverLimit::Stop,
    };

    assert_eq!(
        keep(whole, &[b"abc", b"de", b"fgh"]),
        (b"abcd".to_vec(), true, true)
    );
}

#[test]
fn a_whole_stream_past_its_limit_into_a_sink_never_asks_to_stop() {
    let whole = Capture::Whole {
        limit: 4,
        over: OverLimit::Sink,
    };

    assert_eq!(
        keep(whole, &[b"abcdef", b"gh"]),
        (b"abcd".to_vec(), true, false)
    );
}

#[test]
fn a_tail_keeps_the_last_bytes_up_to_its_limit() {
    let long = vec![b'x'; 3 * SLACK];
    let tail = Capture::Tail { limit: 7 };

    assert_eq!(
        keep(tail, &[&long, b"the end"]),
        (b"the end".to_vec(), true, false)
    );
    assert_eq!(keep(tail, &[b"short"]), (b"short".to_vec(), false, false));
}
