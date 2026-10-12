//! Opt-outs, which let a tool skip one of crumbkit's rules for one command
//! with a stated reason, and the record of which opt-outs a check used.

use clap::Command;

use crate::tree;

#[cfg(test)]
mod tests;

/// One rule of crumbkit's conventions that a tool opts out of for one
/// command and every command below it, with the tool's reason.
///
/// A tool keeps its opt-outs in one constant and passes it to every check
/// that takes opt-outs ([ADR-0036]). Each check reads only the opt-outs of
/// its own rules:
///
/// ```
/// use crumbkit_test::{Allow, Rule};
///
/// const ALLOW: &[Allow] = &[
///     Allow::new(Rule::RootLeaf, "version", "People type `shelf version` from habit."),
///     Allow::new(Rule::OneName, "books", "Scripts written before the rename call `shelf book`."),
/// ];
/// ```
///
/// A check fails on an opt-out of one of its rules that names no command,
/// and on one that excuses no problem in the tree, so the opt-out of a
/// removed or fixed command cannot stay. [`check_opt_outs`] pins the
/// opt-outs, with their reasons and costs, in golden files.
///
/// [`check_opt_outs`]: crate::check_opt_outs
/// [ADR-0036]: https://github.com/crumbkit/crumbkit/blob/main/docs/decisions/api/0036-opt-outs.md
#[derive(Clone, Copy, Debug)]
pub struct Allow {
    /// The rule that the tool skips.
    rule: Rule,
    /// The command's path below the command passed to the check, with its
    /// names joined by single spaces, such as `books list`. An empty path
    /// names the command passed to the check.
    path: &'static str,
    /// Why the tool skips the rule, in one line.
    reason: &'static str,
}

impl Allow {
    /// Returns the opt-out from `rule` for the command at `path` and every
    /// command below it, for `reason`.
    ///
    /// `path` is the command's path below the command passed to the check,
    /// with its names joined by single spaces, such as `"books list"`, and
    /// an empty path names the command passed to the check, so the opt-out
    /// covers the whole tree.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A
    /// `const` that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit_test::{Allow, Rule};
    ///
    /// const BAD: Allow = Allow::new(Rule::RootLeaf, "version", "");
    /// ```
    #[must_use]
    pub const fn new(rule: Rule, path: &'static str, reason: &'static str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self { rule, path, reason }
    }

    /// Returns the rule that the tool skips.
    pub(crate) const fn rule(&self) -> Rule {
        self.rule
    }

    /// Returns the command's path below the command passed to the check.
    pub(crate) const fn path(&self) -> &'static str {
        self.path
    }

    /// Returns the tool's reason.
    pub(crate) const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Reports whether the opt-out covers `rule` for the command at
    /// `below`, its path below the command passed to the check: the
    /// command that the opt-out names or one below it.
    fn covers(&self, rule: Rule, below: &str) -> bool {
        self.rule == rule
            && (self.path.is_empty()
                || below
                    .strip_prefix(self.path)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with(' ')))
    }
}

/// A rule of crumbkit's conventions that a tool can opt out of with an
/// [`Allow`]. Each rule names the check that enforces it.
///
/// The rule of [`check_grammar`](crate::check_grammar) that reserves the
/// word `help` has no opt-out, because crumbkit's `parse` answers `help`
/// itself, and an opt-out could not change that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Rule {
    /// [`check_grammar`](crate::check_grammar): a command without
    /// subcommands sits directly under the tool. Its id is
    /// `grammar/root-leaf`.
    RootLeaf,
    /// [`check_grammar`](crate::check_grammar): a resource's name is not
    /// lowercase words joined by single dashes. Its id is
    /// `grammar/resource-name`.
    ResourceName,
    /// [`check_effects`](crate::check_effects): a visible leaf declares no
    /// effects. Its id is `effects/declared`.
    Effects,
    /// [`check_effects`](crate::check_effects): a command has its own
    /// `--dry-run` in place of crumbkit's `DryRun`. Its id is
    /// `effects/dry-run`.
    DryRun,
    /// [`check_effects`](crate::check_effects): a destructive leaf has no
    /// `--yes`. Its id is `effects/yes`.
    Yes,
    /// [`check_effects`](crate::check_effects): a tool that can ask a
    /// question, because its guide has a picker or a command has `--yes`,
    /// has no `--no-input` from `crumbkit::NoInput` at its root. Its id is
    /// `effects/no-input`, and its opt-out names the root.
    NoInput,
    /// [`check_names`](crate::check_names): help shows a second name for a
    /// command or an argument. Its id is `names/one-name`.
    OneName,
    /// [`check_names`](crate::check_names): the guide names a person's
    /// aliases on the root's help with
    /// `crumbkit::help::Guide::personal_aliases`. Its id is
    /// `names/personal-aliases`, and its opt-out names the root.
    PersonalAliases,
    /// [`check_listings`](crate::check_listings): a `list` command lacks
    /// `--limit` or `--all`. Its id is `listings/list-pages`.
    ListPages,
    /// [`check_listings`](crate::check_listings): a `show` command does not
    /// take from 1 to 100 ids. Its id is `listings/show-ids`.
    ShowIds,
    /// [`check_filters`](crate::check_filters): an option that accepts many
    /// values does not accept both repeats and commas. Its id is
    /// `filters/repeats-and-commas`.
    RepeatsAndCommas,
    /// [`check_filters`](crate::check_filters): one use of an option takes
    /// several values. Its id is `filters/one-value-per-use`.
    OneValuePerUse,
    /// [`check_examples`](crate::check_examples): the check skips the
    /// examples of a command. Its id is `examples/run`.
    Examples,
    /// [`check_document_ids`](crate::check_document_ids): the name of the
    /// document that a command prints is not the command's path below the
    /// tool joined with dashes. Its id is `documents/document-name`.
    DocumentName,
    /// [`check_document_ids`](crate::check_document_ids): a command prints
    /// its document without crumbkit's envelope, because its document
    /// comes from `Document::without_envelope`. Its id is
    /// `documents/envelope`.
    Envelope,
    /// [`check_document_ids`](crate::check_document_ids): the guide sets a
    /// failure form with `crumbkit::help::Guide::failure_form`, so the tool
    /// prints its failures in its own document in place of
    /// `crumbkit/failure/v1`. Its id is `documents/failure-form`, and its
    /// opt-out names the root.
    FailureForm,
    /// [`check_exit_codes`](crate::check_exit_codes): the guide declares
    /// an exit code from 126 to 255, which a shell also reports for its own
    /// reasons. Its id is `exit-codes/shell-code`.
    ShellExitCode,
    /// [`check_recoveries`](crate::check_recoveries): the check runs no
    /// command line below a command and follows no recovery into it. Its id
    /// is `recoveries/follow`.
    Recoveries,
    /// [`check_recoveries`](crate::check_recoveries): a recovery of a
    /// command fails again with the code of the failure it follows. Its id
    /// is `recoveries/same-code`.
    SameCode,
}

impl Rule {
    /// Returns the rule's id, the check's name and the rule's name joined by
    /// `/`, such as `grammar/root-leaf`, which [`check_opt_outs`] prints.
    ///
    /// [`check_opt_outs`]: crate::check_opt_outs
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::RootLeaf => "grammar/root-leaf",
            Self::ResourceName => "grammar/resource-name",
            Self::Effects => "effects/declared",
            Self::DryRun => "effects/dry-run",
            Self::Yes => "effects/yes",
            Self::NoInput => "effects/no-input",
            Self::OneName => "names/one-name",
            Self::PersonalAliases => "names/personal-aliases",
            Self::ListPages => "listings/list-pages",
            Self::ShowIds => "listings/show-ids",
            Self::RepeatsAndCommas => "filters/repeats-and-commas",
            Self::OneValuePerUse => "filters/one-value-per-use",
            Self::Examples => "examples/run",
            Self::DocumentName => "documents/document-name",
            Self::Envelope => "documents/envelope",
            Self::FailureForm => "documents/failure-form",
            Self::ShellExitCode => "exit-codes/shell-code",
            Self::Recoveries => "recoveries/follow",
            Self::SameCode => "recoveries/same-code",
        }
    }

    /// Returns what skipping the rule costs an agent, one sentence each,
    /// which [`check_opt_outs`](crate::check_opt_outs) prints.
    pub(crate) const fn costs(self) -> &'static [&'static str] {
        match self {
            Self::RootLeaf => &[
                "Each such leaf adds a line to the root help, which an agent reads first.",
                "The leaf's name can collide with a resource that the tool adds later.",
                "An agent cannot guess the leaf's siblings from the noun and verb pattern.",
            ],
            Self::ResourceName => {
                &["An agent that spells resources as lowercase dashed words mistypes this name."]
            }
            Self::Effects => &[
                "An undeclared leaf reads as unknown, and an agent must assume the worst.",
                "Help prints no `Effects:` line, and `commands list` writes `null` under `effects`.",
            ],
            Self::DryRun => &[
                "The flag means what the tool makes it mean, which can differ from crumbkit's `--dry-run` in other tools.",
                "The run's JSON document has no `dry_run` field, so an agent cannot tell a dry run from a real one.",
            ],
            Self::Yes => &[
                "An agent without a terminal cannot run the command, or the command runs without asking anyone.",
            ],
            Self::NoInput => &[
                "A script or an agent cannot tell the tool that nobody can answer, so the tool relies on detecting it.",
                "The flag is missing from this tool while other crumbkit tools take it, so a caller that passes it gets a refusal.",
            ],
            Self::OneName => &[
                "Each visible second name costs help bytes on every read of the page.",
                "An agent learns two spellings and can copy either one.",
            ],
            Self::PersonalAliases => &[
                "Root help differs between machines, because each person's aliases file adds its own names.",
                "An agent can copy an alias from help that does not exist on another machine or in a script that another person runs.",
                "Help golden files and help budgets measure help without personal aliases, so a long aliases file can push the root help past its budget unchecked.",
            ],
            Self::ListPages => &[
                "An agent cannot bound the rows that the command prints, so one run can fill its context.",
                "An agent that passes `--limit` or `--all`, as other listings take them, gets a refusal.",
            ],
            Self::ShowIds => &["An agent runs the command once for each id."],
            Self::RepeatsAndCommas => &[
                "An agent that writes the form the option does not accept gets a refusal or a wrong value.",
            ],
            Self::OneValuePerUse => &[
                "One use of the option reads the words after it as more values, so a positional argument after it becomes a value.",
            ],
            Self::Examples => &[
                "The examples are not run, so help can show an example that fails.",
                "The command's output weight counts 0 bytes, so its path weight is too low.",
            ],
            Self::DocumentName => &[
                "An agent cannot predict the document's id from the command it ran, or the command from the id.",
            ],
            Self::Envelope => &[
                "An agent cannot predict the document's shape from crumbkit's conventions and must read its schema with `schemas show`.",
                "The document has no `schema` id, `total` or `failures`, so a program cannot tell which document it read, whether it holds every row, or which ids failed.",
            ],
            Self::FailureForm => &[
                "An agent that knows crumbkit's failure document must learn this tool's failure document, which `schemas show` serves.",
                "The tool owns the schema of its failure document, and changing it breaks its callers unless the document gets a new id.",
                "Each entry of a batch's `failures` keeps crumbkit's shape, so the tool's JSON holds two failure shapes.",
            ],
            Self::ShellExitCode => &[
                "An agent cannot tell the code from the one a shell reports, such as 127 when the shell cannot find the tool or 130 when Ctrl-C ends it.",
            ],
            Self::Recoveries => &[
                "No test shows that the command's recoveries lead anywhere, so a recovery can lead back to the same failure or name a command that no longer runs.",
            ],
            Self::SameCode => &[
                "An agent that follows the recovery sees the same code again and must read the output to learn that the step worked.",
            ],
        }
    }
}

/// The opt-outs of the rules that one check owns, with a record of which
/// of them excused a problem.
pub(crate) struct Allowances<'a> {
    /// The tool's opt-outs, of every rule.
    allow: &'a [Allow],
    /// The rules that the check owns.
    rules: &'a [Rule],
    /// Whether each entry of `allow` excused a problem.
    used: Vec<bool>,
}

impl<'a> Allowances<'a> {
    /// Returns the opt-outs in `allow` of the check that owns `rules`, none
    /// of them used yet.
    pub(crate) fn new(allow: &'a [Allow], rules: &'a [Rule]) -> Self {
        Self {
            allow,
            rules,
            used: vec![false; allow.len()],
        }
    }

    /// Reports whether an opt-out covers `rule` for the command at `below`,
    /// its path below the command passed to the check, and records each
    /// opt-out that does as used. A check calls it only for a problem it
    /// found, so a used opt-out is one that excused a problem.
    pub(crate) fn excuses(&mut self, rule: Rule, below: &str) -> bool {
        let mut excused = false;
        for (allow, used) in self.allow.iter().zip(&mut self.used) {
            if self.rules.contains(&rule) && allow.covers(rule, below) {
                *used = true;
                excused = true;
            }
        }
        excused
    }

    /// Adds a problem to `problems` for each opt-out of the check's rules
    /// that names no command below `root` or excused no problem.
    pub(crate) fn finish(self, root: &Command, problems: &mut Vec<String>) {
        for (allow, used) in self.allow.iter().zip(self.used) {
            if !self.rules.contains(&allow.rule) {
                continue;
            }
            let line = command_line(root, allow.path);
            let id = allow.rule.id();
            if tree::find(root, words(allow.path)).is_none() {
                problems.push(format!(
                    "the opt-out from `{id}` for `{line}` names no command"
                ));
            } else if !used {
                problems.push(format!(
                    "the opt-out from `{id}` for `{line}` excuses no problem, so it is stale"
                ));
            }
        }
    }

    /// Adds the problems of [`Allowances::finish`] to `problems` and
    /// panics, as [`tree::report`] does, when there is any problem. Every
    /// check that takes opt-outs reports through it, so none can leave out
    /// the problems of its opt-outs.
    pub(crate) fn report(self, root: &Command, rules: &str, mut problems: Vec<String>) {
        self.finish(root, &mut problems);
        tree::report(rules, &problems);
    }
}

/// Returns the names in `path`, a command's path below a root with its
/// names joined by single spaces, and no name for an empty path.
pub(crate) fn words(path: &str) -> impl Iterator<Item = &str> {
    path.split(' ').filter(|word| !word.is_empty())
}

/// Returns the command line of `path` below `root`, such as
/// `shelf books list`, or the root's name for an empty path.
pub(crate) fn command_line(root: &Command, path: &str) -> String {
    if path.is_empty() {
        root.get_name().to_owned()
    } else {
        format!("{} {path}", root.get_name())
    }
}

/// Returns whether `text` is non-empty and holds no `\n` or `\r`, as a
/// reason is.
pub(crate) const fn is_reason(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    // A `const fn` cannot call iterator methods, so this loop walks the bytes
    // by index.
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\n' || bytes[index] == b'\r' {
            return false;
        }
        index += 1;
    }
    true
}
