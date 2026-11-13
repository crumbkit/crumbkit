use super::{Capture, Keeper, Kind, LongLine, OverLimit, SLACK};
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
fn the_default_capture_discards_the_stream() {
    assert!(matches!(Capture::default().0, Kind::Discard));
}

#[test]
fn a_discarded_stream_keeps_nothing_and_is_never_over_its_limit() {
    assert_eq!(
        keep(&Capture::discard(), &[b"anything"]),
        (Vec::new(), false, false)
    );
}

#[test]
fn a_whole_stream_up_to_its_limit_is_kept_whole() {
    let whole = Capture::whole(4).into();

    assert_eq!(
        keep(&whole, &[b"ab", b"cd"]),
        (b"abcd".to_vec(), false, false)
    );
}

#[test]
fn a_whole_stream_past_its_limit_keeps_its_first_bytes_and_asks_once_to_stop() {
    let whole = Capture::whole(4).into();

    assert_eq!(
        keep(&whole, &[b"abc", b"de", b"fgh"]),
        (b"abcd".to_vec(), true, true)
    );
}

#[test]
fn a_whole_stream_past_its_limit_into_a_sink_never_asks_to_stop() {
    let whole = Capture::whole(4).over(OverLimit::Sink).into();

    assert_eq!(
        keep(&whole, &[b"abcdef", b"gh"]),
        (b"abcd".to_vec(), true, false)
    );
}

#[test]
fn a_tail_keeps_the_last_bytes_up_to_its_limit() {
    let long = vec![b'x'; 3 * SLACK];
    let tail = Capture::tail(7);

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
    (Capture::lines(queue, total).line(line).into(), lines)
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
fn a_stream_of_lines_with_only_a_total_limit_splits_on_newlines_and_stops_past_the_total() {
    let (queue, lines) = line_queue(64);
    let capture = Capture::lines(queue, 8).into();
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"a\0cdefg\n"));
    assert!(keeper.add(b"h"));
    let kept = keeper.finish();

    assert!(kept.over_limit());
    assert!(!kept.long_line());
    assert_eq!(received(&capture, &lines), [b"a\0cdefg"]);
}

#[test]
fn a_stream_of_lines_into_a_sink_never_asks_to_stop() {
    let (queue, lines) = line_queue(64);
    let capture = Capture::lines(queue, 1024)
        .line(2)
        .over(OverLimit::Sink)
        .into();

    let (_, over, stopped) = keep(&capture, &[b"ab\nabc\nd\n"]);

    assert!(over);
    assert!(!stopped);
    assert_eq!(received(&capture, &lines), [b"ab"]);
}

/// Returns a capture of lines within `line` and `total` bytes that ends each
/// line at `end` and treats a long line as `long` says, and the queue's
/// other side.
fn skipping(line: usize, total: usize, end: u8, long: LongLine) -> (Capture, Lines) {
    let (queue, lines) = line_queue(16);
    let capture = Capture::lines(queue, total).line(line).long(long).end(end);
    (capture.into(), lines)
}

/// Returns what `lines` holds, once its keeper has finished.
fn drained(lines: &Lines) -> Vec<Vec<u8>> {
    std::iter::from_fn(|| lines.recv()).collect()
}

#[test]
fn a_skipped_long_line_is_dropped_and_the_next_lines_still_arrive() {
    let (capture, lines) = skipping(3, 1024, b'\n', LongLine::Skip);
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"one\ntoo long\ntwo\n"));
    let kept = keeper.finish();
    capture.end();

    assert_eq!(drained(&lines), [b"one".to_vec(), b"two".to_vec()]);
    assert!(kept.long_line());
    assert!(!kept.over_limit());
}

#[test]
fn a_long_line_split_across_chunks_is_skipped_whole() {
    let (capture, lines) = skipping(3, 1024, b'\n', LongLine::Skip);
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"abcd"));
    assert!(!keeper.add(b"efgh"));
    assert!(!keeper.add(b"ij\nnext\nok\n"));
    let kept = keeper.finish();
    capture.end();

    assert_eq!(drained(&lines), [b"ok".to_vec()]);
    assert!(kept.long_line());
}

#[test]
fn a_skipped_line_with_no_end_pushes_nothing_when_the_stream_ends() {
    let (capture, lines) = skipping(3, 1024, b'\n', LongLine::Skip);
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"one\ntoo long"));
    let kept = keeper.finish();
    capture.end();

    assert_eq!(drained(&lines), [b"one".to_vec()]);
    assert!(kept.long_line());
    assert!(!kept.over_limit());
}

#[test]
fn a_skipped_line_still_counts_towards_the_total_limit() {
    let (capture, lines) = skipping(3, 8, b'\n', LongLine::Skip);
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"abcd"));
    assert!(keeper.add(b"efghij\n"));
    let kept = keeper.finish();
    capture.end();

    assert_eq!(drained(&lines), Vec::<Vec<u8>>::new());
    assert!(kept.over_limit());
}

#[test]
fn a_line_at_the_limit_is_not_skipped_and_no_long_line_is_reported() {
    let (capture, lines) = skipping(3, 1024, b'\n', LongLine::Skip);
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"abc\n"));
    let kept = keeper.finish();
    capture.end();

    assert_eq!(drained(&lines), [b"abc".to_vec()]);
    assert!(!kept.long_line());
}

#[test]
fn a_custom_end_byte_splits_the_stream_and_a_newline_stays_in_the_line() {
    let (capture, lines) = skipping(16, 1024, 0, LongLine::Over);
    let mut keeper = Keeper::new(&capture);

    assert!(!keeper.add(b"a\nb\0c\0last"));
    let kept = keeper.finish();
    capture.end();

    assert_eq!(
        drained(&lines),
        [b"a\nb".to_vec(), b"c".to_vec(), b"last".to_vec()]
    );
    assert!(!kept.over_limit());
}

#[test]
fn a_long_line_with_the_over_policy_still_takes_the_stream_over_its_limit() {
    let (capture, lines) = skipping(3, 1024, b'\n', LongLine::Over);
    let mut keeper = Keeper::new(&capture);

    assert!(keeper.add(b"too long\nnext\n"));
    let kept = keeper.finish();
    capture.end();

    assert_eq!(drained(&lines), Vec::<Vec<u8>>::new());
    assert!(kept.over_limit());
    assert!(kept.long_line());
}
