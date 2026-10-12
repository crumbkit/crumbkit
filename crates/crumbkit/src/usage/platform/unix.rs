//! The usage log's Unix calls.

use std::fs::{DirBuilder, File, OpenOptions};
use std::io::{self, Read as _};
use std::os::unix::fs::{DirBuilderExt as _, FileExt as _, OpenOptionsExt as _};
use std::path::PathBuf;

use etcetera::BaseStrategy as _;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals as SignalSet;
use signal_hook::low_level::emulate_default_handler;

/// Returns `N` bytes from `/dev/urandom`, or `None` when they cannot be read.
pub(in crate::usage) fn random_bytes<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0; N];
    File::open("/dev/urandom")
        .and_then(|mut random| random.read_exact(&mut bytes))
        .ok()?;
    Some(bytes)
}

/// Returns the XDG state directory, which holds each tool's log directory,
/// or `None` when there is none.
pub(in crate::usage) fn state_home() -> Option<PathBuf> {
    etcetera::choose_base_strategy().ok()?.state_dir()
}

/// Returns the session of a run that no agent gives one: the parent process
/// id, which stays the same for a person typing in one shell.
pub(in crate::usage) fn fallback_session() -> String {
    std::os::unix::process::parent_id().to_string()
}

/// Makes `builder` create directories with mode 0700, so only the user can
/// list the log and its key.
pub(in crate::usage) fn private_dir(builder: &mut DirBuilder) -> &mut DirBuilder {
    builder.mode(0o700)
}

/// Makes `options` create files with mode 0600, so only the user can read
/// them.
pub(in crate::usage) fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    options.mode(0o600)
}

/// Returns the byte of `file` at `offset`, without moving the file's cursor.
pub(in crate::usage) fn read_byte_at(file: &File, offset: u64) -> io::Result<u8> {
    let mut byte = [0];
    file.read_exact_at(&mut byte, offset)?;
    Ok(byte[0])
}

/// The signal-hook iterator over SIGINT and SIGTERM, which the usage log's
/// signal thread reads.
pub(in crate::usage) struct Interrupts(SignalSet);

impl Interrupts {
    /// Takes SIGINT and SIGTERM from now on, or returns `None` when
    /// signal-hook cannot take them.
    pub(in crate::usage) fn listen() -> Option<Self> {
        SignalSet::new([SIGINT, SIGTERM]).ok().map(Self)
    }

    /// Waits for the next signal and returns its number, or `None` when no
    /// more signals can arrive.
    pub(in crate::usage) fn wait(&mut self) -> Option<i32> {
        self.0.forever().next()
    }

    /// Lets `signal` take its default action, which ends the process as it
    /// would have ended without the log.
    pub(in crate::usage) fn end_process(signal: i32) {
        let _ = emulate_default_handler(signal);
    }
}
