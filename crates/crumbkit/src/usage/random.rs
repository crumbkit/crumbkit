//! The random bytes of the usage log's key and of each run's trace ids.

use std::fs::File;
use std::io::Read as _;

/// Returns `N` bytes from `/dev/urandom`, or `None` when they cannot be read.
pub(super) fn bytes<const N: usize>() -> Option<[u8; N]> {
    let mut bytes = [0; N];
    File::open("/dev/urandom")
        .and_then(|mut random| random.read_exact(&mut bytes))
        .ok()?;
    Some(bytes)
}
