//! The settings of the example check, which the twin, recovery and output
//! budget checks share.

use std::path::Path;

use crate::allow::Allow;
use crate::runs::Location;

/// The settings of [`check_examples`](crate::check_examples): the tool's built binary, the
/// directory of the golden files, the environment, XDG fixture, working
/// directory of each run, and the tool's opt-outs.
///
/// A tool declares them as a constant in an integration test of the
/// binary's own package, because cargo sets `CARGO_BIN_EXE_<name>`, the
/// path of the built binary, only for that package's integration tests and
/// benchmarks. An integration test can import only from a library, so the
/// package needs a library target that exports the clap command and the
/// guide, and the binary calls that library:
///
/// ```
/// use crumbkit_test::{Allow, Examples, Rule};
///
/// const ALLOW: &[Allow] = &[Allow::new(Rule::Examples, "books delete", "Each run deletes a book.")];
///
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples")
///     .env(&[("SHELF_DATA", "tests/fixtures/books.json")])
///     .allow(ALLOW);
/// ```
///
/// cargo runs those tests, and therefore the binary, from the package's
/// manifest directory, so a relative path in `dir`, in `current_dir` or in
/// an `.env` value resolves from there.
///
/// `Examples` borrows its paths, variables and opt-outs for `'a`. A
/// constant borrows them for `'static`, and a test that builds a path at
/// runtime, such as a temporary directory, borrows the `String` that holds
/// it.
#[derive(Clone, Copy, Debug)]
pub struct Examples<'a> {
    /// The path of the tool's built binary.
    binary: &'a str,
    /// The directory of the golden files, which holds only example golden
    /// files and the `.stdin` files of examples.
    dir: &'a str,
    /// The environment variables that each run sets, as name and value.
    env: &'a [(&'a str, &'a str)],
    /// The directory whose `config`, `data`, `state` and `cache`
    /// directories each run's fresh XDG directories start as copies of, or
    /// `None` for empty XDG directories.
    xdg: Option<&'a str>,
    /// The directory that each run starts in, or `None` for the test's
    /// directory.
    dir_of_runs: Option<Location<'a>>,
    /// The tool's opt-outs, of which the check reads those from
    /// [`Rule::Examples`](crate::Rule::Examples).
    allow: &'a [Allow],
}

impl<'a> Examples<'a> {
    /// Returns the settings that run the binary at `binary` and keep the
    /// golden files in `dir`, with no environment variables of the tool's
    /// own, no working directory and no opt-out.
    ///
    /// `dir` holds only example golden files and `.stdin` files, because
    /// the check counts every `.txt` and `.stderr` file in it that no
    /// checked example produces as stale, and every `.stdin` file that no
    /// example of the guide names.
    #[must_use]
    pub const fn new(binary: &'a str, dir: &'a str) -> Self {
        Self {
            binary,
            dir,
            env: &[],
            xdg: None,
            dir_of_runs: None,
            allow: &[],
        }
    }

    /// Sets the environment variables that each run sets, each a name and a
    /// value, such as `("SHELF_DATA", "tests/fixtures/books.json")`, so a
    /// tool can point its binary at fixture data or a fake server. The XDG
    /// directories that the check sets win over a variable of the same name,
    /// and [`Examples::xdg`] fills them.
    #[must_use]
    pub const fn env(mut self, env: &'a [(&'a str, &'a str)]) -> Self {
        self.env = env;
        self
    }

    /// Sets the XDG fixture, a directory such as `"tests/fixtures/xdg"` that
    /// holds any of the directories `config`, `data`, `state` and `cache`.
    /// Before each run, the check copies each of them into the run's fresh
    /// directory of `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_STATE_HOME` or
    /// `XDG_CACHE_HOME`, so a tool whose command needs a plugin, a cache or
    /// a configuration file located through those variables finds it, and
    /// no run changes the fixture or what another run sees.
    ///
    /// Every run of [`check_examples`](crate::check_examples),
    /// [`check_output_budget`](crate::check_output_budget)
    /// and [`check_twin`](crate::check_twin), and every page that
    /// [`Pages::binary`](crate::Pages::binary) runs, starts from a new copy, and
    /// each chain of [`check_recoveries`](crate::check_recoveries) copies
    /// once before its start, so its steps see what the earlier runs wrote.
    /// A [`Failing::xdg`](crate::Failing::xdg) wins over this fixture for
    /// its own runs. The check still replaces the path of each XDG directory
    /// with its placeholder, such as `[XDG_DATA_HOME]`.
    ///
    /// A relative `dir` resolves from the test's directory. A check fails
    /// before it runs anything when `dir` is not a directory or holds an
    /// entry other than those four directories, and a run fails when the
    /// fixture holds a symbolic link, which the check does not copy.
    #[must_use]
    pub const fn xdg(mut self, dir: &'a str) -> Self {
        self.xdg = Some(dir);
        self
    }

    /// Sets the directory that each run of an example starts in, such as
    /// `"tests/fixtures/library"`, so a tool whose examples name relative
    /// paths needs no `std::env::set_current_dir`.
    ///
    /// The operating system resolves a relative `dir` from the test's
    /// directory, which cargo sets to the package's manifest directory. The
    /// tool then reads a relative value of [`Examples::env`] from `dir`.
    /// [`check_examples`](crate::check_examples) and
    /// [`check_output_budget`](crate::check_output_budget) run each example
    /// in `dir`, and fail without running any when `dir` is not a
    /// directory. [`check_twin`](crate::check_twin) and
    /// [`check_recoveries`](crate::check_recoveries) run a
    /// [`Failing`](crate::Failing) without a directory of its own in `dir`.
    /// Without this setting, each run starts in the test's directory.
    #[must_use]
    pub const fn current_dir(mut self, dir: &'a str) -> Self {
        self.dir_of_runs = Some(Location::Text(dir));
        self
    }

    /// Sets the tool's opt-outs. The check skips the examples of each
    /// command that an opt-out from [`Rule::Examples`](crate::Rule::Examples) names and of every
    /// command below it, and each example that an
    /// [`Allow::example`] names, and reads no opt-out of another rule, so a
    /// tool passes the same opt-outs that it passes to every other check.
    #[must_use]
    pub const fn allow(mut self, allow: &'a [Allow]) -> Self {
        self.allow = allow;
        self
    }

    /// Returns the path of the tool's built binary.
    pub(crate) const fn binary(&self) -> &'a str {
        self.binary
    }

    /// Returns the directory of the golden files.
    pub(crate) const fn goldens(&self) -> &'a str {
        self.dir
    }

    /// Returns the environment variables that each run sets.
    pub(crate) const fn variables(&self) -> &'a [(&'a str, &'a str)] {
        self.env
    }

    /// Returns the XDG fixture of each run, or `None`.
    pub(crate) fn xdg_fixture(&self) -> Option<&'a Path> {
        self.xdg.map(Path::new)
    }

    /// Returns the directory that each run starts in, or `None`.
    pub(crate) fn working_dir(&self) -> Option<&'a Path> {
        self.dir_of_runs.map(Location::path)
    }

    /// Returns the tool's opt-outs.
    pub(crate) const fn opt_outs(&self) -> &'a [Allow] {
        self.allow
    }
}

/// Returns the problem of a working directory that is
/// not a directory, or of an XDG fixture that breaks its rule, or `None`
/// when `examples` sets none of them or each is sound.
pub(crate) fn setup_problem(examples: &Examples<'_>) -> Option<String> {
    if let Some(dir) = examples.working_dir()
        && !dir.is_dir()
    {
        return Some(format!(
            "Examples::current_dir names `{}`, which is not a directory, so the check runs no example",
            dir.display()
        ));
    }
    examples
        .xdg_fixture()
        .and_then(crate::runs::fixture_problem)
}
