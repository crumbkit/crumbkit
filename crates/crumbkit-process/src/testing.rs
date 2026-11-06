//! Helpers of the tests that run real children, one module for each
//! platform.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(crate) use self::unix::{gone, kill, pid_in, sh};
#[cfg(windows)]
pub(crate) use self::windows::{cmd, gone, pid_file, pid_in, remove, started_grandchild};
