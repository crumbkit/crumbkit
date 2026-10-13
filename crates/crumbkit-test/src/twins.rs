//! The twin check, which runs a command line that a tool refuses beside one
//! that differs from it by one word and succeeds.

use std::path::Path;
use std::process::Output;

use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};

use crate::examples::Examples;
use crate::failures;
use crate::guide::command_line;
use crate::recoveries::{Failing, Unprovoked};
use crate::runs::{self, Dir, Place, Ran, TIMEOUT, WorkDir, Xdg};

// The tests run `/bin/sh` scripts, which Windows cannot run. The two
// attributes stay apart, because clippy reads only a plain `cfg(test)` as
// test code.
#[cfg(unix)]
#[cfg(test)]
mod tests;

/// Runs `refused` and `admitted`, two command lines of the binary of
/// `examples` that differ by one word, for the tool whose guide is `guide`, checks that `refused` fails with its
/// code and that `admitted` exits with code 0, and returns the output of
/// `refused`, so the test can compare it with its golden files. It panics,
/// as a failed test assertion does, when a rule is broken.
///
/// A test of a refusal proves only that the tool fails. Its twin proves
/// that the one word it changes is the reason, because the admitted line
/// without that change succeeds.
///
/// Both runs use the environment of [`Examples::env`], the working
/// directory and standard input of `refused`, or the directory of
/// [`Examples::current_dir`] when `refused` sets none, or a new copy each
/// of the fixture of [`Examples::fresh_dir`], and fresh XDG directories of
/// their own, as [`check_examples`](crate::check_examples) runs an example.
/// The check reads the refused line's code, spelled with `spelling`. When
/// the guide sets a form with `Guide::failure_form`, it reads the code
/// through the form's
/// [`FailureForm::read`](crumbkit::form::FailureForm::read) first, and when
/// `examples` sets a [`FailureReader`](crate::FailureReader), through that
/// reader first. Otherwise, or when neither reads a failure, it reads the
/// code from a `crumbkit/failure/v1` document on standard output, or from
/// the first entry of a batch's `failures`, and then from the last `code:`
/// line on standard error.
///
/// ```no_run
/// use crumbkit::Spelling;
/// use crumbkit::help::Guide;
/// use crumbkit_test::{Examples, Failing};
///
/// const GUIDE: Guide = Guide::new();
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// let output = crumbkit_test::check_twin(
///     &GUIDE,
///     &EXAMPLES,
///     Spelling::COLONS,
///     &Failing::new(&crumbkit::codes::CLI_USAGE, &["books", "list", "--limit", "0"]),
///     &["books", "list", "--limit", "1"],
/// );
/// assert!(output.stdout.is_empty());
/// ```
///
/// # Panics
///
/// Panics when the guide sets a failure form and `examples` sets a failure
/// reader, when the two lines differ by more or less than one word added,
/// removed or replaced, when `refused` exits with code 0, fails with
/// another code or prints no code that the check can read, when `admitted` exits with a code other than 0, or when a
/// run cannot start or does not end within 10 seconds. The message names
/// each line as help prints it, with the binary's file name first.
#[expect(
    clippy::must_use_candidate,
    reason = "a test that compares no output with golden files calls the check only for its assertions"
)]
pub fn check_twin(
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
    admitted: &[&str],
) -> Output {
    match twin(guide, examples, spelling, refused, admitted) {
        Ok(output) => output,
        Err(problem) => {
            #[expect(
                clippy::panic,
                reason = "a check reports a broken rule by panicking, as a failed test assertion does"
            )]
            {
                panic!("{problem}")
            }
        }
    }
}

/// One twin of [`check_twins`]: a command line that the tool refuses, with
/// its code, and the words of a line that differs from it by one word and
/// that the tool accepts.
///
/// ```
/// use crumbkit::Code;
/// use crumbkit_test::{Failing, Twin};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
///
/// const SHOW_7: Twin<'_> = Twin::new(
///     Failing::new(&NOT_FOUND, &["books", "show", "7"]),
///     &["books", "show", "2"],
/// );
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Twin<'a> {
    /// The refused line, with its code.
    refused: Failing<'a>,
    /// The words of the admitted line after the tool's name.
    admitted: &'a [&'a str],
}

impl<'a> Twin<'a> {
    /// Returns the twin of `refused` and `admitted`, the words of the
    /// admitted line after the tool's name.
    #[must_use]
    pub const fn new(refused: Failing<'a>, admitted: &'a [&'a str]) -> Self {
        Self { refused, admitted }
    }
}

/// The settings of [`check_twins`]: how to run the tool's binary, how the
/// tool spells its codes, its declared codes, its twins and the declared
/// codes that no command line in a test can provoke.
///
/// ```
/// use crumbkit::{Code, Spelling};
/// use crumbkit_test::{Examples, Failing, Twin, Twins, Unprovoked};
///
/// const NOT_FOUND: Code = Code::new("shelf", "books", "not_found");
/// const GONE: Code = Code::new("shelf", "books", "removed_file");
///
/// // In an integration test, the binary is `env!("CARGO_BIN_EXE_shelf")`.
/// const EXAMPLES: Examples<'_> = Examples::new("/path/to/shelf", "tests/golden/examples");
///
/// const TWINS: Twins<'_> = Twins::new(EXAMPLES, Spelling::COLONS, &[&[&NOT_FOUND, &GONE]])
///     .pairs(&[Twin::new(
///         Failing::new(&NOT_FOUND, &["books", "show", "7"]),
///         &["books", "show", "2"],
///     )])
///     .unprovoked(&[Unprovoked::new(&GONE, "Only a book file that disappears provokes it.")]);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Twins<'a> {
    /// The binary and its environment.
    examples: Examples<'a>,
    /// How the tool spells its codes.
    spelling: Spelling,
    /// The `CODES` list of each of the tool's error types.
    codes: &'a [&'a [&'a Code]],
    /// The twins.
    pairs: &'a [Twin<'a>],
    /// The declared codes that no command line provokes.
    unprovoked: &'a [Unprovoked<'a>],
}

impl<'a> Twins<'a> {
    /// Returns the settings that run the binary of `examples` with its
    /// environment, spell codes with `spelling` and require a twin for each
    /// code of `codes`, the `CODES` list of each of the tool's error types,
    /// with no twin and no unprovoked code.
    #[must_use]
    pub const fn new(
        examples: Examples<'a>,
        spelling: Spelling,
        codes: &'a [&'a [&'a Code]],
    ) -> Self {
        Self {
            examples,
            spelling,
            codes,
            pairs: &[],
            unprovoked: &[],
        }
    }

    /// Sets the twins.
    #[must_use]
    pub const fn pairs(mut self, pairs: &'a [Twin<'a>]) -> Self {
        self.pairs = pairs;
        self
    }

    /// Sets the declared codes that no command line in a test provokes,
    /// which need no twin. A tool passes the same list to
    /// [`Recoveries::unprovoked`](crate::Recoveries::unprovoked).
    #[must_use]
    pub const fn unprovoked(mut self, unprovoked: &'a [Unprovoked<'a>]) -> Self {
        self.unprovoked = unprovoked;
        self
    }
}

/// Runs every twin of `twins` as [`check_twin`] runs one, for the tool whose
/// guide is `guide`, and panics, as a failed test assertion does, naming
/// every problem it finds (TEST-1).
///
/// It finds:
///
/// - each twin that breaks a rule of [`check_twin`], and it runs every twin
///   whatever an earlier one found;
/// - a twin whose refused line's code is neither a declared code nor one of
///   crumbkit's codes, which it does not run;
/// - a declared code that no twin refuses and [`Twins::unprovoked`] does
///   not list;
/// - a listed code that is not declared or that a twin refuses.
///
/// A twin may refuse with one of crumbkit's codes, such as
/// `crumbkit::cli::usage`, and the check requires no twin for crumbkit's
/// codes.
///
/// # Panics
///
/// Panics when a rule above is broken. The message lists each problem on
/// its own line under `the tool's twins break crumbkit's twin rule:`.
pub fn check_twins(guide: &Guide, twins: &Twins<'_>) {
    let spelling = twins.spelling;
    let declared: Vec<&Code> = twins
        .codes
        .iter()
        .flat_map(|list| list.iter().copied())
        .collect();
    let mut problems = Vec::new();
    for pair in twins.pairs {
        let code = pair.refused.code();
        if !declared.contains(&code) && !crumbkit::codes::ALL.contains(&code) {
            let tool = tool_name(&twins.examples);
            problems.push(format!(
                "the refused line `{}` fails with {}, which is not a declared code or one of crumbkit's codes",
                command_line(&tool, pair.refused.words()),
                spelling.spell(code)
            ));
            continue;
        }
        if let Err(problem) = twin(
            guide,
            &twins.examples,
            spelling,
            &pair.refused,
            pair.admitted,
        ) {
            problems.push(problem);
        }
    }
    let refused = |code: &Code| twins.pairs.iter().any(|pair| pair.refused.code() == code);
    for &code in &declared {
        let listed = twins.unprovoked.iter().any(|entry| entry.code() == code);
        if !listed && !refused(code) {
            problems.push(format!(
                "{}: no twin refuses the declared code, and Twins::unprovoked does not list it",
                spelling.spell(code)
            ));
        }
    }
    for entry in twins.unprovoked {
        let text = spelling.spell(entry.code());
        if !declared.contains(&entry.code()) {
            problems.push(format!(
                "{text}: Twins::unprovoked lists the code, and it is not a declared code"
            ));
        } else if refused(entry.code()) {
            problems.push(format!(
                "{text}: Twins::unprovoked lists the code, and a twin refuses it"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "the tool's twins break crumbkit's twin rule:\n{}",
        problems.join("\n")
    );
}

/// Returns the file name of the binary of `examples` without its
/// extension, which names the tool in a command line.
fn tool_name(examples: &Examples<'_>) -> String {
    Path::new(examples.binary())
        .file_stem()
        .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
}

/// Runs the two lines of [`check_twin`] and returns the output of
/// `refused`, or the problem when a rule is broken.
fn twin(
    guide: &Guide,
    examples: &Examples<'_>,
    spelling: Spelling,
    refused: &Failing<'_>,
    admitted: &[&str],
) -> Result<Output, String> {
    let tool = tool_name(examples);
    let reader = failures::reader(guide, examples)?;
    let refused_line = command_line(&tool, refused.words());
    let admitted_line = command_line(&tool, admitted);
    if !one_word_apart(refused.words(), admitted) {
        return Err(format!(
            "the refused line `{refused_line}` and the admitted line `{admitted_line}` must differ by one word added, removed or replaced"
        ));
    }
    let dir = refused.work_dir(examples.working_dir());
    let stdin = refused.stdin_bytes();
    let code = spelling.spell(refused.code());
    let fixture = refused.fixture(examples.xdg_fixture());
    let output = run(
        examples,
        refused.words(),
        (dir, stdin),
        fixture,
        &refused_line,
    )?;
    if output.status.success() {
        return Err(format!(
            "the refused line `{refused_line}` must fail with {code}, and it exits with code 0"
        ));
    }
    // The first failure is the one the refused line fails with.
    let printed = failures::read_failures(reader, &output.stdout, &output.stderr)
        .first()
        .map(|failure| failure.code().to_owned());
    match printed {
        Some(printed) if printed == code => {}
        Some(printed) => {
            return Err(format!(
                "the refused line `{refused_line}` must fail with {code}, and it fails with {printed}"
            ));
        }
        None => {
            return Err(format!(
                "the refused line `{refused_line}` must fail with {code}, and it ends with {} and prints no code that the check can read. A tool that prints failures in a form of its own implements FailureForm::read and sets the form with Guide::failure_form, or implements FailureReader and sets it with Examples::failure_reader",
                output.status
            ));
        }
    }
    let other = run(examples, admitted, (dir, stdin), fixture, &admitted_line)?;
    if !other.status.success() {
        return Err(format!(
            "the admitted line `{admitted_line}` must exit with code 0, and it ends with {} after writing this to standard error:\n{}",
            other.status,
            String::from_utf8_lossy(&other.stderr)
        ));
    }
    Ok(output)
}

/// Runs `words` with fresh XDG directories that start as copies of
/// `fixture`, and with the working directory and standard input of `start`,
/// a new copy for a fresh fixture, and returns the problem naming `line`
/// when the run cannot start or does not end.
fn run(
    examples: &Examples<'_>,
    words: &[&str],
    start: (Option<Dir<'_>>, Option<&[u8]>),
    fixture: Option<&str>,
    line: &str,
) -> Result<Output, String> {
    let (dir, stdin) = start;
    let ran = Xdg::new("twin", fixture)
        .and_then(|xdg| {
            let work = WorkDir::new("twin-dir", dir)?;
            let place = Place {
                dir: work.path(),
                stdin,
            };
            runs::run(
                examples.binary(),
                examples.variables(),
                &xdg,
                words,
                place,
                TIMEOUT,
            )
        })
        .map_err(|problem| format!("`{line}`: {problem}"))?;
    let Ran::Exited(output) = ran else {
        return Err(format!(
            "`{line}` did not end within {} seconds, so the check stopped it",
            TIMEOUT.as_secs()
        ));
    };
    Ok(output)
}

/// Reports whether `a` and `b` differ by one word added, removed or
/// replaced.
fn one_word_apart(a: &[&str], b: &[&str]) -> bool {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let first = long
        .iter()
        .zip(short)
        .position(|(x, y)| x != y)
        .unwrap_or(short.len());
    match long.len() - short.len() {
        0 => first < long.len() && long[first + 1..] == short[first + 1..],
        1 => long[first + 1..] == short[first..],
        _ => false,
    }
}
