//! The settings of the example check, which the twin, recovery and output
//! budget checks share.

use crumbkit::form::FailureForm;

use crate::allow::Allow;

/// The settings of [`check_examples`](crate::check_examples): the tool's built binary, the
/// directory of the golden files, the environment, working directory and
/// redactions of each run, the tool's opt-outs and its failure form.
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
    /// The directory that each run starts in, or `None` for the test's
    /// directory.
    current_dir: Option<&'a str>,
    /// Each placeholder with the value that it replaces in each output.
    redact: &'a [(&'static str, &'a str)],
    /// The tool's opt-outs, of which the check reads those from
    /// [`Rule::Examples`](crate::Rule::Examples).
    allow: &'a [Allow],
    /// The failure form that the tool's guide sets, which reads back the
    /// failure of a run.
    failure_form: Option<&'a dyn FailureForm>,
}

impl<'a> Examples<'a> {
    /// Returns the settings that run the binary at `binary` and keep the
    /// golden files in `dir`, with no environment variables of the tool's
    /// own, no working directory, no redaction and no opt-out.
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
            current_dir: None,
            redact: &[],
            allow: &[],
            failure_form: None,
        }
    }

    /// Sets the environment variables that each run sets, each a name and a
    /// value, such as `("SHELF_DATA", "tests/fixtures/books.json")`, so a
    /// tool can point its binary at fixture data or a fake server. The XDG
    /// directories that the check sets win over a variable of the same name.
    #[must_use]
    pub const fn env(mut self, env: &'a [(&'a str, &'a str)]) -> Self {
        self.env = env;
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
        self.current_dir = Some(dir);
        self
    }

    /// Sets the values that [`check_examples`](crate::check_examples) replaces with a placeholder
    /// in each output and standard error before it compares them with
    /// their golden files, each a placeholder and a value, such as
    /// `("[ROOT]", root.as_str())` for a temporary directory whose path
    /// changes on every run.
    ///
    /// A placeholder is `[` and `]` around one or more capital letters A to
    /// Z and underscores, as snapbox's `Redactions` requires. The check
    /// matches each value as plain text and replaces longer values first,
    /// and `SNAPSHOTS=overwrite` writes the placeholder into the golden
    /// file. The check fails on a placeholder of another form and on an
    /// empty value. It also replaces the paths of its own XDG directories
    /// with `[XDG_CONFIG_HOME]`, `[XDG_DATA_HOME]`, `[XDG_STATE_HOME]` and
    /// `[XDG_CACHE_HOME]`, because a test cannot know them.
    ///
    /// The golden file holds the placeholder in place of the value, so the
    /// output weight that [`output_weight`](crate::output_weight) reads
    /// from it counts the placeholder's bytes in place of the value's.
    #[must_use]
    pub const fn redact(mut self, redact: &'a [(&'static str, &'a str)]) -> Self {
        self.redact = redact;
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

    /// Sets the failure form that the tool's guide sets with
    /// `Guide::failure_form`, so [`check_twin`](crate::check_twin) reads a
    /// refused run's code through
    /// [`FailureForm::read`](crumbkit::form::FailureForm::read).
    /// [`check_examples`](crate::check_examples) fails when this form and the guide's form differ.
    #[must_use]
    pub const fn failure_form(mut self, form: &'a dyn FailureForm) -> Self {
        self.failure_form = Some(form);
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

    /// Returns the directory that each run starts in, or `None`.
    pub(crate) const fn working_dir(&self) -> Option<&'a str> {
        self.current_dir
    }

    /// Returns each placeholder with the value that it replaces.
    pub(crate) const fn redactions(&self) -> &'a [(&'static str, &'a str)] {
        self.redact
    }

    /// Returns the tool's opt-outs.
    pub(crate) const fn opt_outs(&self) -> &'a [Allow] {
        self.allow
    }

    /// Returns the tool's failure form, or `None`.
    pub(crate) const fn form(&self) -> Option<&'a dyn FailureForm> {
        self.failure_form
    }
}

/// Returns the problem of a working directory that is not a directory, or
/// `None` when `examples` sets none or it is a directory.
pub(crate) fn missing_dir(examples: &Examples<'_>) -> Option<String> {
    let dir = examples.current_dir?;
    (!std::path::Path::new(dir).is_dir()).then(|| {
        format!(
            "Examples::current_dir names `{dir}`, which is not a directory, so the check runs no example"
        )
    })
}
