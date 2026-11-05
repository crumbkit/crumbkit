//! The platform's half of a group: how a child starts, how it is signalled,
//! waited for and reaped, and how a reader waits on a pipe. Each platform
//! module gives the same items, so the rest of the crate holds no platform
//! code.
//!
//! - `Child` holds a started child. It starts it with `spawn`, gives its id
//!   and pipes, signals it, tells whether it has exited without reaping it,
//!   gives a `Waiter`, and reaps it after it kills what is left of its group.
//! - `Waiter` blocks until the child exits, from a thread of its own, and
//!   leaves the child unreaped.
//! - `GRACEFUL` tells whether a stop can send a signal and wait before it
//!   kills.
//! - `exit_of` maps an exit status to an [`Exit`](crate::Exit).
//! - `Pipe` is the trait of a pipe's descriptor or handle, `ready` waits
//!   until a pipe can be read or written, and `unblock` lets a write to a
//!   pipe return before the pipe has room for all of it.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(crate) use self::unix::{Child, GRACEFUL, Pipe, Waiter, exit_of, ready, unblock};
#[cfg(windows)]
pub(crate) use self::windows::{Child, GRACEFUL, Pipe, Waiter, exit_of, ready, unblock};
