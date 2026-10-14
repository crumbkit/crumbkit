//! The check of a tool's twins, which runs every twin and requires one for
//! each declared code.

use clap::Command;
use crumbkit::help::Guide;
use crumbkit::{Code, Spelling};

use super::{Admitted, named, refuse, tool_name, twin};
use crate::allow::{Allow, Rule};
use crate::codes::Declared;
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
    /// The variable whose entry the admitted line runs without, for a twin
    /// made with [`Twin::variable`].
    variable: Option<&'a str>,
}

impl<'a> Twin<'a> {
    /// Returns the twin of `refused` and `admitted`, the words of the
    /// admitted line after the tool's name.
    #[must_use]
    pub const fn new(refused: Failing<'a>, admitted: &'a [&'a str]) -> Self {
        Self {
            refused,
            admitted,
            variable: None,
        }
    }

    /// Returns the twin of `refused` and the same line run without the
    /// entry for the variable `name` that [`Failing::env`] gives `refused`.
    /// The admitted line keeps the refused line's other variables and gets
    /// the variable from [`Examples::env`](crate::Examples::env) or not at
    /// all, as a run without [`Failing::env`] does. The twin shows that the
    /// variable causes the refusal, for a code that the environment
    /// provokes. [`check_twins`] runs it, and [`check_twin`](crate::check_twin)
    /// takes only a line that differs by one word.
    ///
    /// ```
    /// use crumbkit::Code;
    /// use crumbkit_test::{Failing, Twin};
    ///
    /// const TOO_DEEP: Code = Code::new("shelf", "books", "too_deep");
    ///
    /// const DEPTH_1: Twin<'_> = Twin::variable(
    ///     Failing::new(&TOO_DEEP, &["books", "list"]).env(&[("SHELF_DEPTH", Some("1"))]),
    ///     "SHELF_DEPTH",
    /// );
    /// ```
    #[must_use]
    pub const fn variable(refused: Failing<'a>, name: &'a str) -> Self {
        Self {
            refused,
            admitted: refused.words(),
            variable: Some(name),
        }
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
    /// The tool's declared codes.
    codes: Declared<'a>,
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
            codes: Declared::Lists(codes),
            pairs: &[],
            unprovoked: &[],
        }
    }

    /// Returns the settings that [`Twins::new`] returns, for `codes`, one
    /// list of the tool's declared codes held by value, such as codes that
    /// the tool builds at run time.
    #[must_use]
    pub const fn from_codes(examples: Examples<'a>, spelling: Spelling, codes: &'a [Code]) -> Self {
        Self {
            examples,
            spelling,
            codes: Declared::Codes(codes),
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
/// unbuilt command tree is `command` and whose guide is `guide`, and panics, as a failed test assertion does, naming
/// every problem it finds (TEST-1).
///
/// It finds:
///
/// - each twin that breaks a rule of [`check_twin`](crate::check_twin), and it runs every twin
///   whatever an earlier one found;
/// - a twin whose refused line's code is neither a declared code nor one of
///   crumbkit's codes, which it does not run;
/// - a declared code that no twin refuses, [`Twins::unprovoked`] does not
///   list, no opt-out made with [`Allow::untwinned`] names and the guide
///   does not print in place of one of crumbkit's codes;
/// - an untwinned opt-out whose refused line, run with `--json` added,
///   exits with code 0 or fails with another code;
/// - an untwinned opt-out that names a code that is not declared, that a
///   twin refuses, that [`Twins::unprovoked`] lists or that crumbkit owns.
///   The last three are stale.
/// - a listed code that is not declared or that a twin refuses;
/// - a failure code that [`Exit::failures`](crumbkit::help::Exit::failures)
///   lists and no `CODES` list holds.
///
/// Each line runs with `--json` added, as each run of
/// [`check_recoveries`](crate::check_recoveries) does, so the check reads
/// the failure that an agent reads. A twin that holds `--json` in one of its
/// lines runs as written, because then `--json` is the word that the twin
/// changes. [`check_twin`](crate::check_twin) runs its lines as written,
/// because its caller compares the output in the form that the test chose.
///
/// A twin may refuse with one of crumbkit's codes, such as
/// `crumbkit::cli::usage`. Every code in the `crumbkit` component is one of
/// crumbkit's codes, including the codes of crumbkit's add-on crates, such
/// as crumbkit-child's `crumbkit::child::no_json`. The check requires no
/// twin for crumbkit's codes, nor for a declared code that the guide's
/// [`Guide::codes`](crumbkit::help::Guide::codes) prints in place of one of
/// them, because crumbkit raises that failure.
///
/// The check reads the untwinned opt-outs from the
/// [`Examples::allow`](crate::Examples::allow) list of the settings'
/// examples. An untwinned opt-out excuses a declared code that no single
/// word of a command line causes, such as a failure that only a broken file
/// provokes.
///
/// [`Allow::untwinned`]: crate::Allow::untwinned
///
/// # Panics
///
/// Panics when a rule above is broken. The message lists each problem on
/// its own line under `the tool's twins break crumbkit's twin rule:`.
pub fn check_twins(command: &Command, guide: &Guide, twins: &Twins<'_>) {
    let spelling = twins.spelling;
    let declared: Vec<&Code> = twins.codes.to_vec();
    let mut problems = Vec::new();
    for pair in twins.pairs {
        let code = pair.refused.code();
        if !declared.contains(&code) && !crumbkit_owned(guide, code) {
            let tool = tool_name(&twins.examples);
            problems.push(format!(
                "the refused line `{}` fails with {}, which is not a declared code or one of crumbkit's codes",
                command_line(&tool, pair.refused.words()),
                spelling.spell(code)
            ));
            continue;
        }
        if let Err(problem) = run_pair(command, guide, twins, pair) {
            problems.push(problem);
        }
    }
    let refused = |code: &Code| twins.pairs.iter().any(|pair| pair.refused.code() == code);
    let listed = |code: &Code| twins.unprovoked.iter().any(|entry| entry.code() == code);
    let mut untwinned = Vec::new();
    for failing in twins
        .examples
        .opt_outs()
        .iter()
        .filter(|allow| allow.rule() == Rule::NoTwin)
        .filter_map(Allow::refused)
    {
        let code = failing.code();
        let text = spelling.spell(code);
        let line = named(
            &tool_name(&twins.examples),
            failing.words(),
            failing.own_env(),
        );
        let opt_out = format!("the opt-out from `{}` for {line}", Rule::NoTwin.id());
        if crumbkit_owned(guide, code) {
            problems.push(format!(
                "{opt_out} names {text}, one of crumbkit's codes, which need no twin, so it is stale"
            ));
        } else if !declared.contains(&code) {
            problems.push(format!(
                "{opt_out} names {text}, which is not a declared code"
            ));
        } else if refused(code) {
            problems.push(format!(
                "{opt_out} names {text}, which a twin refuses, so it is stale"
            ));
        } else if listed(code) {
            problems.push(format!(
                "{opt_out} names {text}, which Twins::unprovoked lists, so it is stale"
            ));
        } else {
            untwinned.push(code);
            let words = json_lines(failing.words(), &[]).0;
            let words: Vec<&str> = words.iter().map(String::as_str).collect();
            let run = failing.with_words(&words);
            if let Err(problem) = refuse(command, guide, &twins.examples, spelling, &run) {
                problems.push(format!("{opt_out}: {problem}"));
            }
        }
    }
    for &code in &declared {
        if !crumbkit_owned(guide, code)
            && !listed(code)
            && !refused(code)
            && !untwinned.contains(&code)
        {
            problems.push(format!(
                "{}: no twin refuses the declared code, Twins::unprovoked does not list it, and no opt-out made with Allow::untwinned names it",
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
    unheld(guide, spelling, &declared, &mut problems);
    assert!(
        problems.is_empty(),
        "the tool's twins break crumbkit's twin rule:\n{}",
        problems.join("\n")
    );
}

/// Pushes a problem for each failure code that an exit of `guide` lists and
/// `declared`, the codes of the tool's `CODES` lists, does not hold. A
/// listed code of crumbkit's, or the tool's rename of one, needs no `CODES`
/// list, because crumbkit raises the failure.
fn unheld(guide: &Guide, spelling: Spelling, declared: &[&Code], problems: &mut Vec<String>) {
    for exit in guide.get_exits() {
        for &code in exit.get_failures() {
            if !declared.contains(&code)
                && code.component() != "crumbkit"
                && !renamed_to(guide, code)
            {
                problems.push(format!(
                    "{}: the guide's exit code {} lists the failure code, and no CODES list holds it",
                    spelling.spell(code),
                    exit.code()
                ));
            }
        }
    }
}

/// Runs `pair`, one twin of `twins`, with `--json` added as
/// [`json_lines`] adds it, and returns the problem when it breaks a rule of
/// [`check_twin`](crate::check_twin).
fn run_pair(
    command: &Command,
    guide: &Guide,
    twins: &Twins<'_>,
    pair: &Twin<'_>,
) -> Result<(), String> {
    let (refused_words, admitted) = json_lines(pair.refused.words(), pair.admitted);
    let refused_words: Vec<&str> = refused_words.iter().map(String::as_str).collect();
    let admitted: Vec<&str> = admitted.iter().map(String::as_str).collect();
    let refused = pair.refused.with_words(&refused_words);
    let admitted = pair
        .variable
        .map_or(Admitted::Words(&admitted), Admitted::Without);
    twin(
        command,
        guide,
        &twins.examples,
        twins.spelling,
        &refused,
        admitted,
    )
    .map(drop)
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

/// Returns whether `code` is one of crumbkit's codes, or the tool's code
/// that `guide` prints in place of one of them, which crumbkit raises and
/// which needs no twin. Every code in the `crumbkit` component is one of
/// crumbkit's codes, including the codes of crumbkit's add-on crates, such
/// as crumbkit-child, which `crumbkit::codes::ALL` does not list.
fn crumbkit_owned(guide: &Guide, code: &Code) -> bool {
    code.component() == CRUMBKIT || renamed_to(guide, code)
}

/// The component of crumbkit's codes and of its add-on crates' codes.
const CRUMBKIT: &str = "crumbkit";

/// Returns whether `guide` prints `code` in place of one of crumbkit's
/// codes.
fn renamed_to(guide: &Guide, code: &Code) -> bool {
    guide.get_codes().iter().any(|rename| rename.tool() == code)
}
