//! The usage log's Windows calls.

use std::ffi::OsString;
use std::fs::{DirBuilder, File, OpenOptions};
use std::io;
use std::os::windows::fs::FileExt as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use etcetera::BaseStrategy as _;
use signal_hook::consts::SIGINT;

#[cfg(test)]
mod tests;

/// The exit code of a process that Ctrl-C ended, `STATUS_CONTROL_C_EXIT`,
/// as the `i32` that an exit takes.
#[expect(
    clippy::cast_possible_wrap,
    reason = "Windows exit codes are 32-bit, and this one is above `i32::MAX` by design"
)]
const CONTROL_C_EXIT: i32 = 0xC000_013A_u32 as i32;

/// How often the signal thread looks at the flag that the SIGINT handler
/// sets. A person who presses Ctrl-C waits at most this long before the
/// line is written.
const POLL: Duration = Duration::from_millis(50);

/// Returns `N` bytes from the operating system's random generator, or
/// `None` when they cannot be read.
pub(crate) fn random_bytes<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0; N];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes)
}

/// Returns the directory that holds each tool's log directory, or `None`
/// when there is none: `XDG_STATE_HOME` when it is set to an absolute path,
/// and otherwise the local application data directory, `%LOCALAPPDATA%`.
pub(crate) fn state_home() -> Option<PathBuf> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its environment reads, which give its state directory"
    )]
    let xdg = std::env::var_os("XDG_STATE_HOME");
    state_home_from(xdg, || {
        etcetera::choose_base_strategy()
            .ok()
            .map(|strategy| strategy.cache_dir())
    })
}

/// Returns `xdg` when it is an absolute path, and otherwise what `local`
/// returns. The XDG specification says to ignore a relative value.
fn state_home_from(
    xdg: Option<OsString>,
    local: impl FnOnce() -> Option<PathBuf>,
) -> Option<PathBuf> {
    match xdg.map(PathBuf::from) {
        Some(path) if path.is_absolute() => Some(path),
        _ => local(),
    }
}

/// Returns the session of a run that no agent gives one: the Windows
/// Terminal session in `WT_SESSION`, which stays the same for one tab, and
/// otherwise the text `windows`.
//
// RUST-10: outside Windows Terminal every run without an agent shares the
// session `windows`, so `self usage` cannot tell one console from another.
// The parent process id that Unix uses needs a call that crumbkit's ban on
// unsafe code rules out today.
pub(crate) fn fallback_session() -> String {
    #[expect(
        clippy::disallowed_methods,
        reason = "the usage log owns its environment reads, which give the session"
    )]
    let terminal = std::env::var("WT_SESSION").ok();
    session_from(terminal)
}

/// Returns `terminal`, the value of `WT_SESSION`, when it is set and not
/// empty, and otherwise the text `windows`.
fn session_from(terminal: Option<String>) -> String {
    terminal
        .filter(|session| !session.is_empty())
        .unwrap_or_else(|| "windows".to_owned())
}

/// Returns `builder` as it is. The directory inherits the access control
/// list of its parent, which under `%LOCALAPPDATA%` admits only the user,
/// the administrators and the system.
pub(crate) fn private_dir(builder: &mut DirBuilder) -> &mut DirBuilder {
    builder
}

/// Returns `options` as they are. The file inherits the access control
/// list of its directory, as [`private_dir`] describes.
pub(crate) fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    options
}

/// Returns the byte of `file` at `offset`. The read moves the file's
/// cursor, which an append ignores.
pub(crate) fn read_byte_at(file: &File, offset: u64) -> io::Result<u8> {
    let mut byte = [0];
    if file.seek_read(&mut byte, offset)? == 1 {
        Ok(byte[0])
    } else {
        Err(io::ErrorKind::UnexpectedEof.into())
    }
}

/// The flag that the usage log's SIGINT handler sets, which the signal
/// thread reads.
//
// RUST-10: the thread takes only SIGINT, which Ctrl-C raises. Ctrl-Break,
// closing the console, logging off and shutting down end the process with
// no line. Handlers for SIGBREAK and the console's close events replace
// this if a tool needs those lines.
pub(crate) struct Interrupts(Arc<AtomicBool>);

impl Interrupts {
    /// Takes SIGINT from now on, or returns `None` when signal-hook cannot
    /// take it.
    pub(crate) fn listen() -> Option<Self> {
        let raised = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(SIGINT, Arc::clone(&raised)).ok()?;
        Some(Self(raised))
    }

    /// Waits for the next SIGINT and returns its number. The flag is read
    /// every 50 ms, because the handler runs on the console's thread and
    /// can only set it.
    pub(crate) fn wait(&mut self) -> Option<i32> {
        loop {
            if self.0.swap(false, Ordering::SeqCst) {
                return Some(SIGINT);
            }
            thread::sleep(POLL);
        }
    }

    /// Ends the process with the exit code that Ctrl-C gives a process
    /// without a handler, `STATUS_CONTROL_C_EXIT`.
    pub(crate) fn end_process(_signal: i32) {
        signal_hook::low_level::exit(CONTROL_C_EXIT);
    }
}
