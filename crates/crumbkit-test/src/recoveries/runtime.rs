//! Failing command lines that a test builds from data it reads at run time,
//! such as the names of the files in a fixture directory.

use std::path::{Path, PathBuf};

use crumbkit::Code;

use super::Failing;
use crate::allow::Allow;
use crate::twins::Twin;

// The tests run `/bin/sh` scripts, which Windows cannot run. The two
// attributes stay apart, because clippy reads only a plain `cfg(test)` as
// test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// One command line that fails, as a [`Failing`] gives it, built from data
/// that a test reads at run time, such as a file name in a temporary
/// directory. It holds owned words, paths, input, values and variables.
///
/// A test builds one with the same builders as a [`Failing`] and turns it
/// into a [`Failing`] with [`RuntimeFailing::into_failing`], into a
/// [`Twin`] with [`RuntimeFailing::twin`], or into an opt-out from
/// [`Rule::NoTwin`](crate::Rule::NoTwin) with
/// [`RuntimeFailing::untwinned`]. A test passes the results to
/// [`Recoveries::starts`](crate::Recoveries::starts),
/// [`Twins::pairs`](crate::Twins::pairs) and
/// [`Examples::allow`](crate::Examples::allow) as it passes constants:
///
/// ```
/// use crumbkit::Code;
/// use crumbkit_test::{Failing, RuntimeFailing, Twin};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
///
/// let id = 7.to_string();
/// let show = |id: &str| vec!["books".to_owned(), "show".to_owned(), id.to_owned()];
/// let start: Failing<'static> = RuntimeFailing::new(&NOT_FOUND, show(&id)).into_failing();
/// let twin: Twin<'static> = RuntimeFailing::new(&NOT_FOUND, show(&id)).twin(show("2"));
/// ```
///
/// A path of [`RuntimeFailing::dir`] or [`RuntimeFailing::xdg`] need not be
/// UTF-8, and the runs use it unchanged.
///
/// Its builders are not `const fn`s, because each one replaces an owned
/// value, and a `const fn` cannot drop the value it replaces.
#[derive(Clone, Debug)]
pub struct RuntimeFailing {
    /// The code that the command line fails with.
    code: &'static Code,
    /// The words after the tool's name.
    words: Vec<String>,
    /// The working directory of every run of the chain.
    dir: Option<PathBuf>,
    /// The standard input of every run of the chain.
    stdin: Option<String>,
    /// The XDG fixture of the chain.
    xdg: Option<PathBuf>,
    /// The value of each placeholder in the steps of the chain.
    values: Vec<(String, String)>,
    /// The variables that every run of the chain sets or removes.
    env: Vec<(String, Option<String>)>,
}

impl RuntimeFailing {
    /// Returns the command line of `words`, the words after the tool's
    /// name, which fails with `code`, as [`Failing::new`] does.
    #[must_use]
    pub const fn new(code: &'static Code, words: Vec<String>) -> Self {
        Self {
            code,
            words,
            dir: None,
            stdin: None,
            xdg: None,
            values: Vec::new(),
            env: Vec::new(),
        }
    }

    /// Sets the working directory of the command line and of every step of
    /// its chain, as [`Failing::dir`] does.
    #[must_use]
    pub fn dir(mut self, dir: PathBuf) -> Self {
        self.dir = Some(dir);
        self
    }

    /// Sets the text on standard input of the command line and of every
    /// step of its chain, as [`Failing::stdin`] does.
    #[must_use]
    pub fn stdin(mut self, stdin: String) -> Self {
        self.stdin = Some(stdin);
        self
    }

    /// Sets the XDG fixture of the command line and of every step of its
    /// chain, as [`Failing::xdg`] does.
    #[must_use]
    pub fn xdg(mut self, dir: PathBuf) -> Self {
        self.xdg = Some(dir);
        self
    }

    /// Sets the value of each placeholder in the steps of its chain, as
    /// [`Failing::values`] does.
    #[must_use]
    pub fn values(mut self, values: Vec<(String, String)>) -> Self {
        self.values = values;
        self
    }

    /// Sets environment variables of the command line and of every step of
    /// its chain, as [`Failing::env`] does.
    #[must_use]
    pub fn env(mut self, env: Vec<(String, Option<String>)>) -> Self {
        self.env = env;
        self
    }

    /// Returns the [`Failing`] with the same code, words, directory,
    /// input, fixture, values and variables, which the checks run as they
    /// run a constant.
    //
    // RUST-10: `Allow` holds a `'static` refused line, and a `'static`
    // line also lets a test keep its starts and twins anywhere, so this
    // leaks the strings and paths for the rest of the test process. A test
    // calls it once for each command line it builds, and the leak is the
    // size of that line. A `Failing` that borrows from a `RuntimeFailing`
    // replaces this if a process needs to build many lines.
    #[must_use]
    pub fn into_failing(self) -> Failing<'static> {
        let values: Vec<(&'static str, &'static str)> = self
            .values
            .into_iter()
            .map(|(name, value)| (&*name.leak(), &*value.leak()))
            .collect();
        let env: Vec<(&'static str, Option<&'static str>)> = self
            .env
            .into_iter()
            .map(|(name, value)| (&*name.leak(), value.map(|value| &*value.leak())))
            .collect();
        let mut failing = Failing::new(self.code, leak_words(self.words))
            .values(values.leak())
            .env(env.leak());
        if let Some(dir) = self.dir {
            failing = failing.dir_path(leak_path(dir));
        }
        if let Some(stdin) = self.stdin {
            failing = failing.stdin(stdin.leak());
        }
        if let Some(xdg) = self.xdg {
            failing = failing.xdg_path(leak_path(xdg));
        }
        failing
    }

    /// Returns the twin of this refused line and `admitted`, the words of
    /// the admitted line after the tool's name, as [`Twin::new`] does.
    #[must_use]
    pub fn twin(self, admitted: Vec<String>) -> Twin<'static> {
        Twin::new(self.into_failing(), leak_words(admitted))
    }

    /// Returns the opt-out from [`Rule::NoTwin`](crate::Rule::NoTwin) for
    /// the code of this refused line, for `reason`, as [`Allow::untwinned`]
    /// does. The reason may be text that the test builds, such as a
    /// sentence that names a file of a temporary directory.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, as
    /// [`Allow::untwinned`] does.
    #[must_use]
    pub fn untwinned(self, reason: String) -> Allow {
        Allow::untwinned(self.into_failing(), reason.leak())
    }
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
