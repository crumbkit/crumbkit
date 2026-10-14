//! The usage log's calls that differ between Unix and Windows: its random
//! bytes, its state directory, the session of a run without an agent, the
//! permissions of its files, one read at an offset, and its signals.
//!
//! The rest of the log calls these functions and carries no `cfg` of its
//! own. Each platform gives every function below.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(super) use self::unix::{
    Interrupts, fallback_session, private_dir, private_file, random_bytes, read_byte_at, state_home,
};
#[cfg(windows)]
pub(super) use self::windows::{
    Interrupts, fallback_session, private_dir, private_file, random_bytes, read_byte_at, state_home,
};

#[cfg(test)]
mod tests;
