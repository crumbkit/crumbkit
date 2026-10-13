//! `fetch_each`, which runs one lookup per id on a bounded number of threads.

use std::num::NonZeroUsize;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

#[cfg(test)]
mod tests;

/// Calls `lookup` once for each id in `ids`, on at most `workers` threads,
/// and returns one result per id in the order of `ids`.
///
/// A command that takes many ids, such as `shelf books show 2 7 9`, uses it
/// when its backend can look up only one id per call. The command then
/// extends a [`Batch`](crate::Batch) with the results, in order. A repeated
/// id is looked up again, so the ids need no `Hash` or `Eq`. A command that
/// stops once its rows fill the output budget calls `fetch_each` on one
/// chunk of `workers` ids at a time and asks
/// [`Batch::has_room`](crate::Batch::has_room) before each chunk.
///
/// Who should use it, and who should not:
///
/// - A synchronous tool that does blocking I/O for each id, such as a call
///   through a blocking HTTP client, uses it.
/// - A tool whose backend has a batch lookup, such as an SQL `IN` query or an
///   API that takes many ids, calls that lookup once and pushes its answers
///   into a [`Batch`](crate::Batch) in the order of the ids.
/// - An async tool, or one that builds a tokio runtime, uses its own bounded
///   stream, such as `buffered(n)` from the `futures` crate. It must not call
///   `fetch_each` from async code, because `fetch_each` blocks the calling
///   thread until every lookup has returned.
/// - A tool that needs rate limits or retries writes its own loop, because
///   `fetch_each` has neither.
///
/// crumbkit has no default for `workers`, because the right number depends
/// on the backend; the tool chooses it. `fetch_each` starts
/// `min(workers, ids.len())` threads with [`std::thread::scope`]. Each thread
/// takes the next id that no thread has taken yet, so one slow lookup holds
/// up only its own thread. With one worker, one id or no ids, `fetch_each`
/// calls `lookup` in the calling thread and starts no thread.
///
/// ```
/// use std::num::NonZeroUsize;
///
/// const WORKERS: NonZeroUsize = NonZeroUsize::new(4).unwrap();
///
/// let results = crumbkit::fetch_each(&[3, 8, 5], WORKERS, |&id| {
///     if id < 6 { Ok(id * 10) } else { Err(id) }
/// });
///
/// assert_eq!(results, [Ok(30), Err(8), Ok(50)]);
/// ```
///
/// # Panics
///
/// A panic in `lookup` propagates to the caller of `fetch_each` with its
/// original payload, after every thread has stopped. A worker whose lookup
/// panics tells the other workers to stop, and each of them stops before it
/// takes another id. A lookup that is already running finishes first,
/// including its I/O, and its result is dropped.
pub fn fetch_each<I, R, E>(
    ids: &[I],
    workers: NonZeroUsize,
    lookup: impl Fn(&I) -> Result<R, E> + Sync,
) -> Vec<Result<R, E>>
where
    I: Sync,
    R: Send,
    E: Send,
{
    let threads = workers.get().min(ids.len());
    if threads <= 1 {
        return ids.iter().map(lookup).collect();
    }

    // Each worker adds 1 to `next` and looks up the id at the index it read.
    // A worker whose lookup panics sets `stop`, and every worker checks it
    // before it takes an index. `Relaxed` is enough, because the counter only
    // hands out distinct indexes, a worker that reads `stop` late takes at
    // most one more id, and joining a thread makes its results visible to
    // the caller.
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let mut done: Vec<(usize, Result<R, E>)> = Vec::with_capacity(ids.len());
    thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut taken = Vec::new();
                    while !stop.load(Ordering::Relaxed) {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(id) = ids.get(index) else {
                            break;
                        };
                        // The worker passes the panic on at once, and the
                        // other workers stop taking ids, so `lookup` needs
                        // no `UnwindSafe`.
                        match panic::catch_unwind(AssertUnwindSafe(|| lookup(id))) {
                            Ok(result) => taken.push((index, result)),
                            Err(payload) => {
                                stop.store(true, Ordering::Relaxed);
                                panic::resume_unwind(payload);
                            }
                        }
                    }
                    taken
                })
            })
            .collect();
        for handle in handles {
            match handle.join() {
                Ok(taken) => done.extend(taken),
                // A thread that is joined by hand does not make the scope
                // panic, so this passes the lookup's panic on to the caller.
                Err(payload) => panic::resume_unwind(payload),
            }
        }
    });

    // Each worker returns the indexes it took with their results, and sorting
    // by index puts the results back in the order of `ids`. The indexes are
    // distinct, so an unstable sort gives the same order as a stable one.
    done.sort_unstable_by_key(|&(index, _)| index);
    done.into_iter().map(|(_, result)| result).collect()
}
