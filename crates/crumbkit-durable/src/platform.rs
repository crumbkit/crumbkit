//! The calls whose form differs between Unix and Windows: creating the
//! temporary file with its mode, and syncing the directory after the rename.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(crate) use unix::{open_new, sync_directory};
#[cfg(windows)]
pub(crate) use windows::{open_new, sync_directory};
