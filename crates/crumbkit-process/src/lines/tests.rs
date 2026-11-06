use std::time::Duration;

use super::{Silent, line_queue};

/// A wait that ends at once when nothing is queued.
const NOW: Duration = Duration::ZERO;

#[test]
fn lines_come_out_in_the_order_they_went_in_and_none_follows_the_end() {
    let (queue, lines) = line_queue(4);

    assert!(queue.push(b"one".to_vec()));
    assert!(queue.push(b"two".to_vec()));
    queue.end();

    assert_eq!(lines.recv(), Some(b"one".to_vec()));
    assert_eq!(lines.recv(), Some(b"two".to_vec()));
    assert_eq!(lines.recv(), None);
}

#[test]
fn a_wait_with_no_line_and_no_end_is_silent() {
    let (_queue, lines) = line_queue(4);

    assert_eq!(lines.recv_timeout(NOW), Err(Silent));
    assert_eq!(lines.recv_timeout(Duration::from_millis(20)), Err(Silent));
}

#[test]
fn a_wait_too_long_for_the_clock_still_returns_the_next_line() {
    let (queue, lines) = line_queue(4);
    assert!(queue.push(b"line".to_vec()));

    assert_eq!(
        lines.recv_timeout(Duration::MAX),
        Ok(Some(b"line".to_vec()))
    );
}

#[test]
fn a_full_queue_waits_until_the_caller_takes_a_line() {
    let (queue, lines) = line_queue(1);
    assert!(queue.push(b"first".to_vec()));
    let pusher = std::thread::spawn({
        let queue = queue.clone();
        move || queue.push(b"second".to_vec())
    });
    std::thread::sleep(Duration::from_millis(50));
    assert!(!pusher.is_finished(), "the push did not wait for room");

    assert_eq!(lines.recv(), Some(b"first".to_vec()));

    assert!(pusher.join().unwrap());
    assert_eq!(lines.recv_timeout(NOW), Ok(Some(b"second".to_vec())));
}

#[test]
fn a_full_queue_after_a_stop_drops_each_line_and_keeps_the_last_as_lost() {
    let (queue, lines) = line_queue(1);
    assert!(queue.push(b"kept".to_vec()));
    queue.stopping();

    assert!(queue.push(b"dropped".to_vec()));
    assert!(queue.push(b"last".to_vec()));
    assert_eq!(lines.lost(), None, "the run has not ended yet");
    queue.end();

    assert_eq!(lines.recv(), Some(b"kept".to_vec()));
    assert_eq!(lines.recv(), None);
    assert_eq!(lines.lost(), Some(b"last".to_vec()));
    assert_eq!(lines.lost(), None);
}

#[test]
fn a_line_queued_after_a_lost_one_makes_the_lost_one_no_longer_last() {
    let (queue, lines) = line_queue(1);
    assert!(queue.push(b"kept".to_vec()));
    queue.stopping();
    assert!(queue.push(b"dropped".to_vec()));
    assert_eq!(lines.recv(), Some(b"kept".to_vec()));

    assert!(queue.push(b"last".to_vec()));
    queue.end();

    assert_eq!(lines.recv(), Some(b"last".to_vec()));
    assert_eq!(lines.lost(), None);
}

#[test]
fn a_push_after_the_caller_dropped_its_side_returns_false_and_never_waits() {
    let (queue, lines) = line_queue(1);
    assert!(queue.push(b"first".to_vec()));
    let pusher = std::thread::spawn({
        let queue = queue.clone();
        move || queue.push(b"second".to_vec())
    });
    std::thread::sleep(Duration::from_millis(50));

    drop(lines);

    assert!(!pusher.join().unwrap());
    assert!(!queue.push(b"third".to_vec()));
}

#[test]
fn a_queue_of_no_lines_holds_one() {
    let (queue, lines) = line_queue(0);
    queue.stopping();

    assert!(queue.push(b"one".to_vec()));
    assert!(queue.push(b"two".to_vec()));
    queue.end();

    assert_eq!(lines.recv(), Some(b"one".to_vec()));
    assert_eq!(lines.lost(), Some(b"two".to_vec()));
}
