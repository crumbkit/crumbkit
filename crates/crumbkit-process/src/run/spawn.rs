//! The hook around the start of a run's child.

use std::fmt;
use std::io;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use crate::{Group, Scope};

/// The start of one run's child, which a hook set with
/// [`RunOptions::around_spawn`](crate::RunOptions::around_spawn) receives.
#[derive(Debug)]
pub struct Spawn {
    command: Command,
    scope: Scope,
}

impl Spawn {
    /// Starts the child.
    ///
    /// # Errors
    ///
    /// Returns the error of the start, as [`RunError::Start`](crate::RunError::Start)
    /// describes it.
    pub fn start(self) -> io::Result<Spawned> {
        let started = Instant::now();
        let group = Group::start(self.command, self.scope)?;
        Ok(Spawned { group, started })
    }
}

/// A child that [`Spawn::start`] started, which a hook returns to the run.
///
/// Only [`Spawn::start`] makes this value, so a hook that returns `Ok` has
/// started the child. Dropping it kills the child and every process in its
/// scope and reaps the child.
#[derive(Debug)]
#[must_use = "dropping a started child kills it"]
pub struct Spawned {
    group: Group,
    started: Instant,
}

impl Spawned {
    /// Returns the child and the time just before it started.
    pub(super) fn into_parts(self) -> (Group, Instant) {
        (self.group, self.started)
    }
}

/// A hook around the start of a run's child.
#[derive(Clone)]
pub(super) struct AroundSpawn(Hook);

/// The caller's function, which threads of a run share with the settings.
type Hook = Arc<dyn Fn(Spawn) -> io::Result<Spawned> + Send + Sync>;

impl AroundSpawn {
    /// Wraps the caller's function.
    pub(super) fn new(hook: impl Fn(Spawn) -> io::Result<Spawned> + Send + Sync + 'static) -> Self {
        Self(Arc::new(hook))
    }
}

impl fmt::Debug for AroundSpawn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AroundSpawn(..)")
    }
}

/// Starts `command` in `scope`, through `hook` when the run has one.
pub(super) fn start(
    command: Command,
    scope: Scope,
    hook: Option<&AroundSpawn>,
) -> io::Result<Spawned> {
    let spawn = Spawn { command, scope };
    match hook {
        Some(AroundSpawn(hook)) => hook(spawn),
        None => spawn.start(),
    }
}
