//! Failing command lines that a test builds from data it reads at run time,
//! such as the names of the files in a fixture directory.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use crumbkit::Code;

use super::Failing;
use crate::allow::Allow;
use crate::twins::Twin;

mod place;

pub use place::RuntimePlace;

// The tests run `/bin/sh` scripts, which Windows cannot run. The two
// attributes stay apart, because clippy reads only a plain `cfg(test)` as
// test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// One command line that fails, as a [`Failing`] gives it, built from data
/// that a test reads at run time, such as a file name in a temporary
/// directory or a code that the tool builds at run time. It holds its code
/// and owned words, paths, input, values and variables.
///
/// A test builds one with builders like those of a [`Failing`] and turns
/// it into a [`Failing`] with [`RuntimeFailing::into_failing`], into a
/// [`Twin`] with [`RuntimeFailing::twin`] or
/// [`RuntimeFailing::variable_twin`], or into an opt-out from
/// [`Rule::NoTwin`](crate::Rule::NoTwin) with
/// [`RuntimeFailing::untwinned`]. A test passes the results to
/// [`Recoveries::starts`](crate::Recoveries::starts),
/// [`Twins::pairs`](crate::Twins::pairs) and
/// [`Examples::allow`](crate::Examples::allow) as it passes constants.
/// [`RuntimeFailing::line`] gives several command lines one directory,
/// input, fixture, values and variables, and [`RuntimeFailing::at`] makes
/// a [`RuntimePlace`] that holds only those settings:
///
/// ```
/// use crumbkit::Code;
/// use crumbkit_test::{Allow, Failing, RuntimeFailing, Twin};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
/// const UNREADABLE: Code = Code::new("shelf", "books", "unreadable");
///
/// let id = 7.to_string();
/// let place = RuntimeFailing::at("tests/fixtures/shelf").env("SHELF_DEPTH", "1");
/// let start: Failing<'static> = place.line(NOT_FOUND, ["books", "show", &id]).into_failing();
/// let twin: Twin<'static> = place
///     .line(NOT_FOUND, ["books", "show", &id])
///     .twin(["books", "show", "2"]);
/// let allow: Allow = place
///     .line(UNREADABLE, ["books", "list"])
///     .untwinned("The shelf file cannot be read, and no word of the line names that.");
/// ```
///
/// A path of [`RuntimeFailing::dir`] or [`RuntimeFailing::xdg`] need not be
/// UTF-8, and the runs use it unchanged.
///
/// Its builders are not `const fn`s, because each one takes or replaces an
/// owned value, and a `const fn` cannot drop the value it replaces.
#[derive(Clone, Debug)]
pub struct RuntimeFailing {
    /// The code that the command line fails with.
    code: Code,
    /// The words after the tool's name.
    words: Vec<String>,
    /// The directory, input, fixture, values and variables of the chain.
    settings: Settings,
}

/// The settings that a [`RuntimeFailing`] and a [`RuntimePlace`] share.
#[derive(Clone, Debug, Default)]
struct Settings {
    /// The working directory of every run of the chain.
    dir: Option<PathBuf>,
    /// The standard input of every run of the chain.
    stdin: Option<String>,
    /// The XDG fixture of the chain.
    xdg: Option<PathBuf>,
    /// The value of each placeholder in the steps of the chain.
    values: Vec<(String, String)>,
    /// The variables that every run of the chain sets or removes, in the
    /// order the test gave them.
    env: Vec<(String, Option<String>)>,
}

impl RuntimeFailing {
    /// Returns the command line of `words`, the words after the tool's
    /// name, which fails with `code`, as [`Failing::new`] does. The code may
    /// be one that the tool builds at run time.
    #[must_use]
    pub fn new(code: Code, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            code,
            words: owned(words),
            settings: Settings::default(),
        }
    }

    /// Returns a place: the working directory `dir` with no input, fixture,
    /// values or variables, and with no code and no words. A test adds the
    /// other settings with the builders of [`RuntimePlace`] and builds each
    /// command line from the place with [`RuntimePlace::line`].
    ///
    /// A place has no command line of its own, so a test that passes a
    /// place where a command line belongs does not compile:
    ///
    /// ```compile_fail
    /// use crumbkit_test::RuntimeFailing;
    ///
    /// let start = RuntimeFailing::at("tests/fixtures/shelf").into_failing();
    /// ```
    #[must_use]
    pub fn at(dir: impl Into<PathBuf>) -> RuntimePlace {
        RuntimePlace::new(Settings {
            dir: Some(dir.into()),
            ..Settings::default()
        })
    }

    /// Returns the command line of `words` that fails with `code`, with the
    /// directory, input, fixture, values and variables of `self`, so one
    /// set of settings serves several command lines.
    #[must_use]
    pub fn line(&self, code: Code, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::with_settings(code, words, self.settings.clone())
    }

    /// Returns the command line of `words` that fails with `code`, with
    /// `settings`.
    fn with_settings(
        code: Code,
        words: impl IntoIterator<Item = impl Into<String>>,
        settings: Settings,
    ) -> Self {
        Self {
            code,
            words: owned(words),
            settings,
        }
    }

    /// Sets the working directory of the command line and of every step of
    /// its chain, as [`Failing::dir`] does.
    #[must_use]
    pub fn dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.settings.dir = Some(dir.into());
        self
    }

    /// Returns the working directory that [`RuntimeFailing::dir`] or the
    /// place of [`RuntimePlace::line`] set, or `None` when neither set one.
    /// The name has clap's `get_` prefix, because `dir` is the builder
    /// method.
    #[must_use]
    pub fn get_dir(&self) -> Option<&Path> {
        self.settings.dir.as_deref()
    }

    /// Sets the text on standard input of the command line and of every
    /// step of its chain, as [`Failing::stdin`] does.
    #[must_use]
    pub fn stdin(mut self, stdin: impl Into<String>) -> Self {
        self.settings.stdin = Some(stdin.into());
        self
    }

    /// Sets the XDG fixture of the command line and of every step of its
    /// chain, as [`Failing::xdg`] does.
    #[must_use]
    pub fn xdg(mut self, dir: impl Into<PathBuf>) -> Self {
        self.settings.xdg = Some(dir.into());
        self
    }

    /// Sets the XDG fixture to `dir` when it is `Some`, as
    /// [`RuntimeFailing::xdg`] does, and removes the fixture when it is
    /// `None`, so the runs use the fixture of
    /// [`Examples::xdg`](crate::Examples::xdg) or none.
    #[must_use]
    pub fn xdg_option(mut self, dir: Option<PathBuf>) -> Self {
        self.settings.xdg = dir;
        self
    }

    /// Sets the value of each placeholder in the steps of its chain, as a
    /// name and a value, as [`Failing::values`] does.
    #[must_use]
    pub fn values(
        mut self,
        values: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.settings.values = owned_pairs(values);
        self
    }

    /// Adds the variable `name` with `value` to the variables that the
    /// command line and every step of its chain set, as an entry of
    /// [`Failing::env`] does. A later entry for the same name wins.
    #[must_use]
    pub fn env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.settings.env.push((name.into(), Some(value.into())));
        self
    }

    /// Adds the variable `name` to the variables that the command line and
    /// every step of its chain remove, as an entry of [`Failing::env`] with
    /// `None` does. A later entry for the same name wins.
    #[must_use]
    pub fn env_remove(mut self, name: impl Into<String>) -> Self {
        self.settings.env.push((name.into(), None));
        self
    }

    /// Returns the [`Failing`] with the same code, words, directory,
    /// input, fixture, values and variables, which the checks run as they
    /// run a constant.
    //
    // RUST-10: `Allow` holds a `'static` refused line, and a `'static`
    // line also lets a test keep its starts and twins anywhere, so this
    // leaks the code, strings and paths for the rest of the test process. A test
    // calls it once for each command line it builds, and the leak is the
    // size of that line and of its code. A `Failing` that borrows from a `RuntimeFailing`
    // replaces this if a process needs to build many lines.
    #[must_use]
    pub fn into_failing(self) -> Failing<'static> {
        let Settings {
            dir,
            stdin,
            xdg,
            values,
            env,
        } = self.settings;
        let values: Vec<(&'static str, &'static str)> = values
            .into_iter()
            .map(|(name, value)| (&*name.leak(), &*value.leak()))
            .collect();
        let env: Vec<(&'static str, Option<&'static str>)> = env
            .into_iter()
            .map(|(name, value)| (&*name.leak(), value.map(|value| &*value.leak())))
            .collect();
        let code: &'static Code = Box::leak(Box::new(self.code));
        let mut failing = Failing::new(code, leak_words(self.words))
            .values(values.leak())
            .env(env.leak());
        if let Some(dir) = dir {
            failing = failing.dir_path(leak_path(dir));
        }
        if let Some(stdin) = stdin {
            failing = failing.stdin(stdin.leak());
        }
        if let Some(xdg) = xdg {
            failing = failing.xdg_path(leak_path(xdg));
        }
        failing
    }

    /// Returns the twin of this refused line and `admitted`, the words of
    /// the admitted line after the tool's name, as [`Twin::new`] does.
    #[must_use]
    pub fn twin(self, admitted: impl IntoIterator<Item = impl Into<String>>) -> Twin<'static> {
        Twin::new(self.into_failing(), leak_words(owned(admitted)))
    }

    /// Returns the twin of this refused line and the same line run without
    /// the entry for the variable `name` that [`RuntimeFailing::env`] or
    /// [`RuntimeFailing::env_remove`] gives it, as [`Twin::variable`] does.
    /// The name may be text that the test builds.
    #[must_use]
    pub fn variable_twin(self, name: impl Into<String>) -> Twin<'static> {
        Twin::variable(self.into_failing(), name.into().leak())
    }

    /// Returns the opt-out from [`Rule::NoTwin`](crate::Rule::NoTwin) for
    /// the code of this refused line, for `reason`, as [`Allow::untwinned`]
    /// does. The reason may be a constant, which the opt-out keeps as it
    /// is, or text that the test builds, such as a sentence that names a
    /// file of a temporary directory, which the opt-out leaks.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, as
    /// [`Allow::untwinned`] does.
    #[must_use]
    pub fn untwinned(self, reason: impl Into<Cow<'static, str>>) -> Allow {
        let reason: &'static str = match reason.into() {
            Cow::Borrowed(reason) => reason,
            Cow::Owned(reason) => reason.leak(),
        };
        Allow::untwinned(self.into_failing(), reason)
    }
}

/// Returns `words` as owned strings.
fn owned(words: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
    words.into_iter().map(Into::into).collect()
}

/// Returns `pairs` as owned names and values.
fn owned_pairs(
    pairs: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
) -> Vec<(String, String)> {
    pairs
        .into_iter()
        .map(|(name, value)| (name.into(), value.into()))
        .collect()
}

/// Returns `words` as a `'static` slice of `'static` words, leaking them.
fn leak_words(words: Vec<String>) -> &'static [&'static str] {
    let words: Vec<&'static str> = words.into_iter().map(|word| &*word.leak()).collect();
    words.leak()
}

/// Returns `path` as a `'static` path, leaking it.
fn leak_path(path: PathBuf) -> &'static Path {
    Box::leak(path.into_boxed_path())
}
