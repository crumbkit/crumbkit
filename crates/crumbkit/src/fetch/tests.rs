//! Tests for `fetch_each`.

use std::collections::HashSet;
use std::num::NonZeroUsize;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Barrier, Mutex};
use std::thread::{self, ThreadId};
use std::time::Duration;

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
fn lookups_that_finish_out_of_order_still_return_in_the_order_of_the_ids() {
    // The first id sleeps longest and each later id sleeps less, so the lookups
    // finish in roughly the reverse of the order they started.
    let ids: Vec<u64> = (0..8).collect();

    let results = fetch_each(&ids, workers(8), |&id| {
        thread::sleep(Duration::from_millis(5 * (8 - id)));
        Ok::<u64, ()>(id)
    });

    assert_eq!(results, ids.into_iter().map(Ok).collect::<Vec<_>>());
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
fn more_workers_than_ids_start_at_most_one_thread_per_id() {
    let threads = Mutex::new(HashSet::new());

    let results = fetch_each(&[1, 2, 3], workers(16), |&id| {
        // Each lookup sleeps, so every worker that starts has time to take an
        // id and record its thread.
        thread::sleep(Duration::from_millis(20));
        threads.lock().unwrap().insert(thread::current().id());
        Ok::<u32, ()>(id)
    });

    assert_eq!(results, [Ok(1), Ok(2), Ok(3)]);
    let threads = threads.into_inner().unwrap();
    assert!(threads.len() <= 3, "{} threads ran lookups", threads.len());
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

#[test]
fn a_panic_in_a_lookup_stops_the_other_workers_before_they_take_another_id() {
    let ids: Vec<u32> = (0..100).collect();
    let started = Barrier::new(2);
    let calls = AtomicUsize::new(0);

    // Ids 0 and 1 go to different workers, because the worker that takes id 0
    // panics in its lookup. Each lookup after the panic takes 1 ms, so a
    // worker that kept going would make about 100 calls.
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        fetch_each(&ids, workers(2), |&id| {
            calls.fetch_add(1, Ordering::Relaxed);
            if id < 2 {
                started.wait();
            }
            assert_ne!(id, 0, "the lookup of id 0 panicked");
            thread::sleep(Duration::from_millis(1));
            Ok::<u32, ()>(id)
        })
    }));

    assert!(result.is_err());
    assert!(calls.into_inner() < 10);
}
