//! Tests for `fetch_each`.

use std::cell::RefCell;
use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Barrier, Mutex};
use std::thread::{self, ThreadId};

use super::fetch_each;

/// Returns `count` as a worker count.
fn workers(count: usize) -> NonZeroUsize {
    NonZeroUsize::new(count).unwrap()
}

/// Looks up `id` and returns it doubled, or the id itself as the error when
/// it is odd.
fn double_even(id: u64) -> Result<u64, u64> {
    if id.is_multiple_of(2) {
        Ok(id * 2)
    } else {
        Err(id)
    }
}

#[test]
fn many_workers_return_one_result_per_id_in_the_order_of_the_ids() {
    let ids: Vec<u64> = (0..50).collect();

    let results = fetch_each(&ids, workers(4), |&id| double_even(id));

    let expected: Vec<Result<u64, u64>> = ids.iter().map(|&id| double_even(id)).collect();
    assert_eq!(results, expected);
}

#[test]
fn ids_that_two_workers_take_in_turn_still_return_in_the_order_of_the_ids() {
    // The lookups of each pair of neighbouring ids meet at a barrier, so a
    // lookup cannot return until the next id's lookup has started. That
    // lookup must run on the other worker, which makes the two workers take
    // the ids in turn: one takes 0, 2 and 4, and the other takes 1, 3 and 5.
    // Each worker's own results are therefore out of order with the
    // other's, whichever worker the caller joins first.
    let ids: Vec<usize> = (0..6).collect();
    let neighbours: Vec<Barrier> = (1..ids.len()).map(|_| Barrier::new(2)).collect();
    let threads = Mutex::new(Vec::new());

    let results = fetch_each(&ids, workers(2), |&id| {
        if id > 0 {
            neighbours[id - 1].wait();
        }
        if let Some(next) = neighbours.get(id) {
            next.wait();
        }
        threads.lock().unwrap().push((id, thread::current().id()));
        Ok::<usize, ()>(id)
    });

    assert_eq!(results, ids.iter().copied().map(Ok).collect::<Vec<_>>());
    let mut threads = threads.into_inner().unwrap();
    threads.sort_unstable_by_key(|&(id, _)| id);
    for pair in threads.windows(2) {
        assert_ne!(
            pair[0].1, pair[1].1,
            "ids {} and {} ran on one worker",
            pair[0].0, pair[1].0
        );
    }
}

#[test]
fn one_worker_runs_every_lookup_in_the_calling_thread() {
    let caller = thread::current().id();

    let results = fetch_each(&[1, 2, 3], workers(1), |_| {
        Ok::<ThreadId, ()>(thread::current().id())
    });

    assert_eq!(results, [Ok(caller), Ok(caller), Ok(caller)]);
}

#[test]
fn one_id_runs_in_the_calling_thread_whatever_the_number_of_workers() {
    let caller = thread::current().id();

    let results = fetch_each(&[1], workers(8), |_| {
        Ok::<ThreadId, ()>(thread::current().id())
    });

    assert_eq!(results, [Ok(caller)]);
}

#[test]
fn more_workers_than_ids_run_each_lookup_on_a_worker_thread_of_its_own() {
    // The three lookups wait for each other at a barrier, so none of them
    // returns until all three are running at once on three threads.
    let all_running = Barrier::new(3);
    let threads = Mutex::new(HashSet::new());

    let results = fetch_each(&[1, 2, 3], workers(16), |&id| {
        all_running.wait();
        threads.lock().unwrap().insert(thread::current().id());
        Ok::<u32, ()>(id)
    });

    assert_eq!(results, [Ok(1), Ok(2), Ok(3)]);
    let threads = threads.into_inner().unwrap();
    assert_eq!(threads.len(), 3);
    assert!(!threads.contains(&thread::current().id()));
}

#[test]
fn no_ids_return_no_results_and_call_no_lookup() {
    let calls = AtomicUsize::new(0);

    let results = fetch_each(&[] as &[u32], workers(4), |&id| {
        calls.fetch_add(1, Ordering::Relaxed);
        Ok::<u32, ()>(id)
    });

    assert_eq!(results, Vec::new());
    assert_eq!(calls.into_inner(), 0);
}

#[test]
fn a_repeated_id_is_looked_up_again_and_returns_its_result_again() {
    let calls = AtomicUsize::new(0);

    let results = fetch_each(&[7, 7, 2, 7], workers(3), |&id| {
        calls.fetch_add(1, Ordering::Relaxed);
        Ok::<u32, ()>(id * 10)
    });

    assert_eq!(results, [Ok(70), Ok(70), Ok(20), Ok(70)]);
    assert_eq!(calls.into_inner(), 4);
}

#[test]
#[should_panic(expected = "the lookup of id 3 panicked")]
fn a_panic_in_a_lookup_on_a_worker_thread_propagates_to_the_caller() {
    let ids: Vec<u32> = (0..6).collect();

    let _ = fetch_each(&ids, workers(3), |&id| {
        assert_ne!(id, 3, "the lookup of id 3 panicked");
        Ok::<u32, ()>(id)
    });
}

#[test]
#[should_panic(expected = "the lookup of id 2 panicked")]
fn a_panic_in_a_lookup_in_the_calling_thread_propagates_to_the_caller() {
    let _ = fetch_each(&[1, 2], workers(1), |&id| {
        assert_ne!(id, 2, "the lookup of id 2 panicked");
        Ok::<u32, ()>(id)
    });
}

/// Sends one message on its channel when it drops.
struct SendOnDrop(Sender<()>);

impl Drop for SendOnDrop {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

thread_local! {
    /// Holds a `SendOnDrop` for a thread that should report its own exit.
    /// Rust drops a thread's locals when the thread exits.
    static ON_EXIT: RefCell<Option<SendOnDrop>> = const { RefCell::new(None) };
}

#[test]
fn a_panic_in_a_lookup_stops_the_other_workers_before_they_take_another_id() {
    let ids: Vec<u32> = (0..100).collect();
    let both_running = Barrier::new(2);
    let (exited, panicked_worker_exited) = mpsc::channel();
    let panicked_worker_exited = Mutex::new(panicked_worker_exited);
    let calls = AtomicUsize::new(0);

    // The lookups of ids 0 and 1 meet at a barrier, so they run on the two
    // workers at once. The worker with id 0 panics, tells the other worker to
    // stop, and then exits, which drops its `ON_EXIT` and sends on the
    // channel. The lookup of id 1 returns only after that message, so the
    // other worker checks whether to stop after the panicked worker has told
    // it to.
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        fetch_each(&ids, workers(2), |&id| {
            calls.fetch_add(1, Ordering::Relaxed);
            match id {
                0 => {
                    ON_EXIT.with(|on_exit| {
                        *on_exit.borrow_mut() = Some(SendOnDrop(exited.clone()));
                    });
                    both_running.wait();
                    panic!("the lookup of id 0 panicked");
                }
                1 => {
                    both_running.wait();
                    panicked_worker_exited.lock().unwrap().recv().unwrap();
                }
                _ => {}
            }
            Ok::<u32, ()>(id)
        })
    }));

    assert!(result.is_err());
    assert_eq!(calls.into_inner(), 2);
}
