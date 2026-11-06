//! Tests that run real children. The Unix tests run `sh` scripts and the
//! Windows tests run `cmd` scripts, so neither needs a test program of its
//! own.

#[cfg(unix)]
mod lines;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
