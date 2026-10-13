//! The check of a tool's twins, which runs every twin and requires one for
//! each declared code.

use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};

use super::{tool_name, twin};
use crate::examples::Examples;
use crate::guide::command_line;
use crate::recoveries::{Failing, Unprovoked};
use crate::runs::with_json;

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

/// Runs every twin of `twins` as [`check_twin`](crate::check_twin) runs one, for the tool whose
/// guide is `guide`, and panics, as a failed test assertion does, naming
/// every problem it finds (TEST-1).
///
/// It finds:
///
/// - each twin that breaks a rule of [`check_twin`](crate::check_twin), and it runs every twin
///   whatever an earlier one found;
/// - a twin whose refused line's code is neither a declared code nor one of
///   crumbkit's codes, which it does not run;
/// - a declared code that no twin refuses and [`Twins::unprovoked`] does
///   not list;
/// - a listed code that is not declared or that a twin refuses.
///
/// Each line runs with `--json` added, as each run of
/// [`check_recoveries`](crate::check_recoveries) does, so the check reads
/// the failure that an agent reads. A twin that holds `--json` in one of its
/// lines runs as written, because then `--json` is the word that the twin
/// changes. [`check_twin`](crate::check_twin) runs its lines as written,
/// because its caller compares the output in the form that the test chose.
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
        let (refused_words, admitted) = json_lines(pair.refused.words(), pair.admitted);
        let refused_words: Vec<&str> = refused_words.iter().map(String::as_str).collect();
        let admitted: Vec<&str> = admitted.iter().map(String::as_str).collect();
        let refused = pair.refused.with_words(&refused_words);
        if let Err(problem) = twin(guide, &twins.examples, spelling, &refused, &admitted) {
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

/// Returns the words of the refused and the admitted line of a twin as
/// [`check_twins`] runs them: each with `--json` added when neither line
/// holds it, or both as written.
fn json_lines(refused: &[&str], admitted: &[&str]) -> (Vec<String>, Vec<String>) {
    let owned = |words: &[&str]| {
        words
            .iter()
            .map(|&word| word.to_owned())
            .collect::<Vec<_>>()
    };
    let (refused, admitted) = (owned(refused), owned(admitted));
    let holds = |words: &[String]| words.iter().any(|word| word == "--json");
    if holds(&refused) || holds(&admitted) {
        (refused, admitted)
    } else {
        (with_json(&refused), with_json(&admitted))
    }
}
