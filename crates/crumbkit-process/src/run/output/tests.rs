use super::{Capture, Keeper, OverLimit, SLACK};
use crate::{Lines, line_queue};

/// Returns what `capture` kept of `chunks`, and whether any chunk asked to
/// stop the child.
fn keep(capture: &Capture, chunks: &[&[u8]]) -> (Vec<u8>, bool, bool) {
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
        keep(&Capture::Discard, &[b"anything"]),
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
        keep(&whole, &[b"ab", b"cd"]),
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
        keep(&whole, &[b"abc", b"de", b"fgh"]),
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
        keep(&whole, &[b"abcdef", b"gh"]),
        (b"abcd".to_vec(), true, false)
    );
}

#[test]
fn a_tail_keeps_the_last_bytes_up_to_its_limit() {
    let long = vec![b'x'; 3 * SLACK];
    let tail = Capture::Tail { limit: 7 };

    assert_eq!(
        keep(&tail, &[&long, b"the end"]),
        (b"the end".to_vec(), true, false)
    );
    assert_eq!(keep(&tail, &[b"short"]), (b"short".to_vec(), false, false));
}

/// Returns a capture of lines within `line` and `total` bytes that stops
/// the child past them, and the caller's side of its queue.
fn lines(line: usize, total: usize) -> (Capture, Lines) {
    let (queue, lines) = line_queue(64);
    let capture = Capture::Lines {
        queue,
        line,
        total,
        over: OverLimit::Stop,
    };
    (capture, lines)
}

/// Returns every line that `lines` holds once its run has ended.
fn received(capture: &Capture, lines: &Lines) -> Vec<Vec<u8>> {
    capture.end();
    std::iter::from_fn(|| lines.recv()).collect()
}

#[test]
fn a_stream_of_lines_is_split_on_each_newline_across_reads_and_ends_with_its_last_line() {
    let (capture, queue) = lines(64, 1024);

    let (bytes, over, stopped) = keep(&capture, &[b"one\ntw", b"o\n\nthr", b"ee"]);

    assert_eq!((bytes, over, stopped), (Vec::new(), false, false));
    assert_eq!(
        received(&capture, &queue),
        [&b"one"[..], b"two", b"", b"three"]
    );
}

#[test]
fn a_line_longer_than_its_limit_asks_once_to_stop_and_no_later_line_is_pushed() {
    let (capture, queue) = lines(4, 1024);
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"four\nfive"));
    assert!(keeper.add(b"s\nsix\n"));
    assert!(!keeper.add(b"seven\n"));
    let kept = keeper.finish();

    assert!(kept.over_limit());
    assert!(kept.long_line());
    assert_eq!(received(&capture, &queue), [b"four"]);
}

#[test]
fn a_stream_longer_than_its_total_limit_is_over_it_without_a_long_line() {
    let (capture, queue) = lines(64, 10);
    let mut keeper = Keeper::new(&capture);

    assert!(keeper.add(b"aaaa\nbbbb\ncccc\n"));
    let kept = keeper.finish();

    assert!(kept.over_limit());
    assert!(!kept.long_line());
    assert_eq!(received(&capture, &queue), [b"aaaa", b"bbbb"]);
}

#[test]
fn a_stream_of_lines_into_a_sink_never_asks_to_stop() {
    let (queue, lines) = line_queue(64);
    let capture = Capture::Lines {
        queue,
        line: 2,
        total: 1024,
        over: OverLimit::Sink,
    };

    let (_, over, stopped) = keep(&capture, &[b"ab\nabc\nd\n"]);

    assert!(over);
    assert!(!stopped);
    assert_eq!(received(&capture, &lines), [b"ab"]);
}
