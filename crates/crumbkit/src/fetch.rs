//! `fetch_each`, which runs one lookup per id on a bounded number of threads.

use std::num::NonZeroUsize;
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

#[cfg(test)]
mod tests;

/// Calls `lookup` once for each id in `ids`, on at most `workers` threads,
/// and returns one result per id in the order of `ids`.
///
/// A command that takes many ids, such as `shelf books show 2 7 9`, uses it
/// when its backend can look up only one id per call. The command then puts
/// the `Ok` values and the failures it builds from the `Err` values into a
/// [`Batch`](crate::Batch). A repeated id is looked up again, so the ids need
/// no `Hash` or `Eq`.
///
/// Who should use it, and who should not:
///
/// - A synchronous tool that does blocking I/O for each id, such as a call
///   through a blocking HTTP client, uses it.
/// - A tool whose backend has a batch lookup, such as an SQL `IN` query or an
///   API that takes many ids, calls that lookup once and builds
///   [`Batch::new`](crate::Batch::new) from its answer.
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
/// original payload, after every thread has stopped, as a panic in a thread
/// of [`std::thread::scope`] does.
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
    // `Relaxed` is enough, because the counter only hands out distinct
    // indexes, and joining a thread makes its results visible to the caller.
    let next = AtomicUsize::new(0);
    let mut done: Vec<(usize, Result<R, E>)> = Vec::with_capacity(ids.len());
    thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut taken = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(id) = ids.get(index) else {
                            return taken;
                        };
                        taken.push((index, lookup(id)));
                    }
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
