//! The facts that `shelf books show` adds to shelf's usage log.

use crumbkit_usage::{Extension, Facts, Key};

use super::{Book, BooksError};

/// The number of `lookups` in shelf's usage facts.
const LOOKUPS: Key = Key::new("lookups");

/// The number of `missing` books in shelf's usage facts.
const MISSING: Key = Key::new("missing");

/// What the lookups of `shelf books show` did, which shelf's usage log
/// records as facts: how many ids it looked up and how many the catalogue
/// lacked. A tool that calls a network API would also record the time it
/// waited for the server with `Facts::waited` (ADR-0144).
#[derive(Default)]
pub(super) struct Lookups {
    /// The ids looked up.
    looked_up: u64,
    /// The ids that the catalogue lacked or that were removed.
    missing: u64,
}

impl Lookups {
    /// Adds the lookups of one chunk, whose results `found` holds.
    pub(super) fn add(&mut self, found: &[Result<Book, BooksError>]) {
        for result in found {
            self.looked_up += 1;
            self.missing += u64::from(result.is_err());
        }
    }
}

impl Extension for Lookups {
    fn write(&self, facts: &mut Facts) {
        facts
            .count(LOOKUPS, self.looked_up)
            .count(MISSING, self.missing);
    }
}
