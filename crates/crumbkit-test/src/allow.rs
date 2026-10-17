//! Opt-outs, which let a tool skip one of crumbkit's rules for one command
//! with a stated reason, and the record of which opt-outs a check used.

use clap::Command;

use crate::recoveries::Failing;

mod allowances;
#[cfg(test)]
mod tests;

pub(crate) use allowances::Allowances;

/// One rule of crumbkit's conventions that a tool opts out of for one
/// command and every command below it, with the tool's reason.
///
/// A tool keeps its opt-outs in one constant and passes it to every check
/// that takes opt-outs ([opt-outs docs]). Each check reads only the opt-outs of
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
/// [opt-outs docs]: https://github.com/crumbkit/crumbkit/blob/main/docs/behaviour/adoption/opt-outs.md
#[derive(Clone, Copy, Debug)]
pub struct Allow {
    /// The rule that the tool skips.
    rule: Rule,
    /// The command's path below the command passed to the check, with its
    /// names joined by single spaces, such as `books list`. An empty path
    /// names the command passed to the check.
    path: &'static str,
    /// The words of the one example that the opt-out covers, after the
    /// command's path, or `None` for an opt-out that covers every command
    /// below `path`.
    words: Option<&'static [&'static str]>,
    /// The refused command line of an opt-out from [`Rule::NoTwin`], or
    /// `None` for an opt-out that names a command.
    refused: Option<Failing<'static>>,
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
    ///
    /// It also panics for [`Rule::NoTwin`], whose opt-out names a refused
    /// command line with [`Allow::untwinned`], because a command's path
    /// names no code:
    ///
    /// ```compile_fail
    /// use crumbkit_test::{Allow, Rule};
    ///
    /// const BAD: Allow = Allow::new(Rule::NoTwin, "books show", "A broken file causes it.");
    /// ```
    #[must_use]
    pub const fn new(rule: Rule, path: &'static str, reason: &'static str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        assert!(
            !matches!(rule, Rule::NoTwin),
            "an opt-out from twins/no-twin names its refused line with Allow::untwinned"
        );
        Self {
            rule,
            path,
            words: None,
            refused: None,
            reason,
        }
    }

    /// Returns the opt-out from [`Rule::Examples`] for the one example of
    /// the leaf at `path` whose words are `words`, for `reason`, so the
    /// other examples of the leaf still run.
    ///
    /// `path` is the leaf's path below the command passed to the check,
    /// such as `"books list"`, and `words` are the example's words after
    /// it, exactly as the guide's `Example::new` declares them, such as
    /// `&["--author", "Jane Austen"]`. [`check_opt_outs`] names the opt-out
    /// by the example's command line, such as
    /// `shelf books list --author 'Jane Austen'`. The example check fails on
    /// an example opt-out whose path names no command or whose words name
    /// no example that it runs.
    ///
    /// ```
    /// use crumbkit_test::Allow;
    ///
    /// const ALLOW: &[Allow] = &[Allow::example("self usage", &["--days", "7"], "The output names the day.")];
    /// ```
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, as
    /// [`Allow::new`] does.
    ///
    /// [`check_opt_outs`]: crate::check_opt_outs
    #[must_use]
    pub const fn example(
        path: &'static str,
        words: &'static [&'static str],
        reason: &'static str,
    ) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            rule: Rule::Examples,
            path,
            words: Some(words),
            refused: None,
            reason,
        }
    }

    /// Returns the opt-out from [`Rule::NoTwin`] for the code of `refused`,
    /// a command line that fails with that code, for `reason`, so
    /// [`check_twins`] needs no twin for the code.
    ///
    /// A twin shows which one word of a command line causes a failure. Some
    /// failures come from state that no word names, such as a list file
    /// that cannot be read. The check runs `refused` and requires it to
    /// fail with its code, so the opt-out shows that a test provokes the
    /// code. [`check_opt_outs`] names the opt-out by the refused command
    /// line.
    ///
    /// ```
    /// use crumbkit::Code;
    /// use crumbkit_test::{Allow, Failing};
    ///
    /// const UNREADABLE: Code = Code::new("reading", "lists", "unreadable");
    ///
    /// const ALLOW: &[Allow] = &[Allow::untwinned(
    ///     Failing::new(&UNREADABLE, &["entries", "list", "--list", "locked"]),
    ///     "The list file cannot be read, and no word of the command line names that.",
    /// )];
    /// ```
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, as
    /// [`Allow::new`] does.
    ///
    /// [`check_twins`]: crate::check_twins
    /// [`check_opt_outs`]: crate::check_opt_outs
    #[must_use]
    pub const fn untwinned(refused: Failing<'static>, reason: &'static str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            rule: Rule::NoTwin,
            path: "",
            words: None,
            refused: Some(refused),
            reason,
        }
    }

    /// Returns the refused command line of an opt-out from
    /// [`Rule::NoTwin`], or `None`.
    pub(crate) const fn refused(&self) -> Option<Failing<'static>> {
        self.refused
    }

    /// Returns the rule that the tool skips.
    pub(crate) const fn rule(&self) -> Rule {
        self.rule
    }

    /// Returns the tool's reason.
    pub(crate) const fn reason(&self) -> &'static str {
        self.reason
    }

    /// Returns the command line that the opt-out covers below `root`, such
    /// as `shelf books list`, or the example's command line, such as
    /// `shelf books list --author 'Jane Austen'`.
    pub(crate) fn line(&self, root: &Command) -> String {
        if let Some(refused) = self.refused {
            return crate::guide::command_line(root.get_name(), refused.words());
        }
        let line = command_line(root, self.path);
        match self.words {
            Some(words) => crate::guide::command_line(&line, words),
            None => line,
        }
    }

    /// Returns what skipping the rule costs an agent, one sentence each:
    /// the costs of the rule, or those of skipping one example.
    pub(crate) const fn costs(&self) -> &'static [&'static str] {
        if self.words.is_some() {
            &[
                "The example is not run, so help can show an example that fails.",
                "The example's output is not counted, so the command's output weight is too low when this example prints the most.",
            ]
        } else {
            self.rule.costs()
        }
    }

    /// Reports whether the opt-out covers `rule` for the command at
    /// `below`, its path below the command passed to the check: the
    /// command that the opt-out names or one below it.
    fn covers(&self, rule: Rule, below: &str) -> bool {
        self.rule == rule
            && self.words.is_none()
            && self.refused.is_none()
            && (self.path.is_empty()
                || below
                    .strip_prefix(self.path)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with(' ')))
    }

    /// Reports whether the opt-out covers the one example `words` of the
    /// leaf at `below`.
    fn covers_example(&self, below: &str, words: &[&str]) -> bool {
        self.rule == Rule::Examples && self.path == below && self.words == Some(words)
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
    /// lowercase words joined by single dashes, and the resource answers to
    /// no such dashed form. Its id is `grammar/resource-name`. A resource
    /// named in another style, such as `reading_lists`, needs no opt-out
    /// when the guide declares `Guide::dashed_forms` and the name maps to a
    /// dashed form that no other command beside it takes.
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
    /// [`check_effects`](crate::check_effects): a leaf whose effects are
    /// `crumbkit::help::Effects::replacing` has no `--expect` from
    /// `crumbkit::Expect`. Its id is `effects/expect`.
    Expect,
    /// [`check_effects`](crate::check_effects): a command has its own
    /// `--expect` in place of crumbkit's `Expect`. Its id is
    /// `effects/expect-flag`.
    ExpectFlag,
    /// [`check_effects`](crate::check_effects): a leaf that writes takes
    /// many items in a positional argument and declares neither
    /// `all_or_none` nor `per_item` on its effects. Its id is
    /// `effects/items`.
    Items,
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
    /// examples of a command, or one example that
    /// [`Allow::example`] names. Its id is `examples/run`.
    Examples,
    /// [`check_document_ids`](crate::check_document_ids): a command prints
    /// its document without crumbkit's envelope, because its document has
    /// `Envelope::Without` or `Envelope::WithoutInOrder`. Its id is
    /// `documents/envelope`.
    Envelope,
    /// [`check_document_ids`](crate::check_document_ids): a command prints
    /// every field of each row in JSON without `--full`, because its row
    /// type sets `Row::WHOLE`. Its id is `documents/rows-whole`.
    RowsWhole,
    /// [`check_exit_codes`](crate::check_exit_codes): the guide declares
    /// an exit code from 126 to 255, which a shell also reports for its own
    /// reasons. Its id is `exit-codes/shell-code`.
    ShellExitCode,
    /// [`check_exit_codes`](crate::check_exit_codes): the guide declares
    /// the exit code 1 or 2, which already mean crumbkit's default failure
    /// and a refused command line. Its id is `exit-codes/shared-exit`.
    SharedExit,
    /// [`check_recoveries`](crate::check_recoveries): the check runs no
    /// command line below a command and follows no recovery into it. Its id
    /// is `recoveries/follow`.
    Recoveries,
    /// [`check_recoveries`](crate::check_recoveries): a recovery of a
    /// command fails with the code of the failure it follows or of an
    /// earlier failure of its chain. Its id is `recoveries/same-code`, and
    /// its opt-out names the command that the recovery runs, which marks a
    /// step that a person or an agent must complete by hand first.
    SameCode,
    /// [`check_recoveries`](crate::check_recoveries): a recovery of a
    /// command holds a placeholder that a field of the command's failure
    /// fills, so an agent must copy the value itself. Its id is
    /// `recoveries/field-value`, and its opt-out names the command whose
    /// failure printed the recovery.
    FieldValue,
    /// [`check_recoveries`](crate::check_recoveries): a failure with one of
    /// the tool's declared codes offers no `run`, `retry` or `hint` step,
    /// such as a failure whose recovery is `Recovery::None`. Its id is
    /// `recoveries/next-step`, and its opt-out names the command that
    /// failed. The opt-out declares the exception that ADR-0001 allows, so
    /// the reason shows in the report of
    /// [`check_opt_outs`](crate::check_opt_outs).
    NextStep,
    /// [`check_twins`](crate::check_twins): a declared code that a command
    /// line provokes has no twin, because no single word of the line causes
    /// it. Its id is `twins/no-twin`, and its opt-out, made with
    /// [`Allow::untwinned`], names the refused command line.
    NoTwin,
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
            Self::Expect => "effects/expect",
            Self::ExpectFlag => "effects/expect-flag",
            Self::Items => "effects/items",
            Self::OneName => "names/one-name",
            Self::PersonalAliases => "names/personal-aliases",
            Self::ListPages => "listings/list-pages",
            Self::ShowIds => "listings/show-ids",
            Self::RepeatsAndCommas => "filters/repeats-and-commas",
            Self::OneValuePerUse => "filters/one-value-per-use",
            Self::Examples => "examples/run",
            Self::Envelope => "documents/envelope",
            Self::RowsWhole => "documents/rows-whole",
            Self::ShellExitCode => "exit-codes/shell-code",
            Self::SharedExit => "exit-codes/shared-exit",
            Self::Recoveries => "recoveries/follow",
            Self::SameCode => "recoveries/same-code",
            Self::FieldValue => "recoveries/field-value",
            Self::NextStep => "recoveries/next-step",
            Self::NoTwin => "twins/no-twin",
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
            Self::ResourceName => &[
                "An agent that spells resources as lowercase dashed words cannot reach this resource by that spelling.",
            ],
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
            Self::Expect => &[
                "Two concurrent writers can lose each other's changes, because the command replaces data without checking that it is the version the caller read.",
            ],
            Self::ExpectFlag => &[
                "The flag means what the tool makes it mean, which can differ from crumbkit's `--expect` in other tools.",
                "A failed check does not fail with `crumbkit::version::conflict`, so an agent cannot tell a conflict from another failure by its code.",
            ],
            Self::Items => &[
                "An agent that retries after a failure cannot tell which items the first run changed.",
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
            Self::Envelope => &[
                "An agent cannot predict the document's shape from crumbkit's conventions and must read its schema with `schemas show`.",
                "The document has no `schema` id, `truncated` or `failures`, so a program cannot tell which document it read, whether it holds every row, or which ids failed.",
            ],
            Self::RowsWhole => &[
                "--json prints every field of each row, so the document costs more tokens than the compact columns.",
            ],
            Self::ShellExitCode => &[
                "An agent cannot tell the code from the one a shell reports, such as 127 when the shell cannot find the tool or 130 when Ctrl-C ends it.",
            ],
            Self::SharedExit => &[
                "The exit code has two meanings, so an agent must read the failure's code to learn which one applies.",
            ],
            Self::Recoveries => &[
                "No test shows that the command's recoveries lead anywhere, so a recovery can lead back to the same failure or name a command that no longer runs.",
            ],
            Self::SameCode => &[
                "An agent that follows the recovery sees a code that its chain already had and must read the output to learn that the step worked.",
            ],
            Self::FieldValue => &[
                "An agent that follows the recovery must copy the value from the failure's field into the placeholder itself, where a recovery built with `Call::value` would carry the value.",
            ],
            Self::NextStep => &[
                "An agent that reads the failure has no command, retry or advice to act on, so it must stop or guess.",
            ],
            Self::NoTwin => &[
                "No test shows which word of the command line causes the failure, so a test of the refusal can pass while the command fails for another reason.",
            ],
        }
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
