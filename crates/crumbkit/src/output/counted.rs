//! A writer that counts the bytes it passes on, which `finish` puts in front
//! of standard output so it can report them to the observer.

use std::io::{self, Write};

#[cfg(test)]
mod tests;

/// A writer that passes every write to `inner` and counts the bytes that
/// `inner` took.
pub(crate) struct Counted<W> {
    /// The writer that receives the bytes.
    inner: W,
    /// The bytes that `inner` took so far.
    bytes: u64,
}

impl<W: Write> Counted<W> {
    /// Returns a writer over `inner` that has counted no bytes.
    pub(crate) const fn new(inner: W) -> Self {
        Self { inner, bytes: 0 }
    }

    /// Returns the number of bytes that the inner writer took.
    pub(crate) const fn bytes(&self) -> u64 {
        self.bytes
    }
}

impl<W: Write> Write for Counted<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
