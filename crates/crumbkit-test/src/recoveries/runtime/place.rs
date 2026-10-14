//! A place: the settings that several failing command lines share, without
//! a command line of its own.

use std::path::{Path, PathBuf};

use crumbkit::Code;

use super::{RuntimeFailing, Settings, owned_pairs};

/// A working directory, input, fixture, values and variables that several
/// [`RuntimeFailing`] command lines share, made with
/// [`RuntimeFailing::at`].
///
/// A place has no code and no words, so it has no method that turns it
/// into a [`Failing`](crate::Failing), a [`Twin`](crate::Twin) or an
/// [`Allow`](crate::Allow). A test builds each command line from it with
/// [`RuntimePlace::line`], and the compiler refuses a place passed where a
/// command line belongs. Its builders set what the builders of the same
/// name on [`RuntimeFailing`] set.
#[derive(Clone, Debug)]
pub struct RuntimePlace {
    /// The directory, input, fixture, values and variables of each line.
    settings: Settings,
}

impl RuntimePlace {
    /// Returns the place that holds `settings`.
    pub(super) const fn new(settings: Settings) -> Self {
        Self { settings }
    }

    /// Returns the command line of `words` that fails with `code`, with the
    /// directory, input, fixture, values and variables of the place, as
    /// [`RuntimeFailing::new`] does with those settings added.
    #[must_use]
    pub fn line(
        &self,
        code: Code,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> RuntimeFailing {
        RuntimeFailing::with_settings(code, words, self.settings.clone())
    }

    /// Sets the working directory of each line, as
    /// [`RuntimeFailing::dir`] does.
    #[must_use]
    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.settings.dir = Some(dir.into());
        self
    }

    /// Returns the working directory that [`RuntimeFailing::at`] or
    /// [`RuntimePlace::dir`] set.
    #[must_use]
    pub fn get_dir(&self) -> Option<&Path> {
        self.settings.dir.as_deref()
    }

    /// Sets the text on standard input of each line, as
    /// [`RuntimeFailing::stdin`] does.
    #[must_use]
    pub fn stdin(mut self, stdin: impl Into<String>) -> Self {
        self.settings.stdin = Some(stdin.into());
        self
    }

    /// Sets the XDG fixture of each line, as [`RuntimeFailing::xdg`] does.
    #[must_use]
    pub fn xdg(mut self, dir: impl Into<PathBuf>) -> Self {
        self.settings.xdg = Some(dir.into());
        self
    }

    /// Sets or removes the XDG fixture of each line, as
    /// [`RuntimeFailing::xdg_option`] does.
    #[must_use]
    pub fn xdg_option(mut self, dir: Option<PathBuf>) -> Self {
        self.settings.xdg = dir;
        self
    }

    /// Sets the value of each placeholder in the steps of each line's
    /// chain, as [`RuntimeFailing::values`] does.
    #[must_use]
    pub fn values(
        mut self,
        values: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.settings.values = owned_pairs(values);
        self
    }

    /// Adds the variable `name` with `value` to the variables of each line,
    /// as [`RuntimeFailing::env`] does.
    #[must_use]
    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.settings.env.push((name.into(), Some(value.into())));
        self
    }

    /// Adds the variable `name` to the variables that each line removes, as
    /// [`RuntimeFailing::env_remove`] does.
    #[must_use]
    pub fn env_remove(mut self, name: impl Into<String>) -> Self {
        self.settings.env.push((name.into(), None));
        self
    }
}
