//! The calls whose form differs between Unix and Windows: creating the
//! temporary file with its mode, syncing the directory after the rename,
//! and opening, identifying and checking the owner of a lock file.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(crate) use unix::{
    Caller, Identity, check_owner, file_identity, open_lock, open_new, path_identity,
    sync_directory,
};
#[cfg(windows)]
pub(crate) use windows::{
    Caller, Identity, check_owner, file_identity, open_lock, open_new, path_identity,
    sync_directory,
};
