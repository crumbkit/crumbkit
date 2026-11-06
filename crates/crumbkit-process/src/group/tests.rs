//! Tests of a group's start, signals and reap with real children. The Unix
//! tests run `sh` scripts and the Windows tests run `cmd` scripts.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
