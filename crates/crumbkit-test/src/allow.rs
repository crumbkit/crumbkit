//! Opt-outs, which let a tool skip one of crumbkit's rules for one command
//! with a stated reason, and the record of which opt-outs a check used.

use clap::Command;

use crate::recoveries::Failing;

mod allowances;
mod rule;
#[cfg(test)]
mod tests;

pub(crate) use allowances::Allowances;
pub use rule::Rule;

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
    /// The one option of the command at `path` that the opt-out covers, as
    /// help names it, such as `--author`, or `None` for an opt-out that
    /// covers every option.
    option: Option<&'static str>,
    /// The refused command line of an opt-out from [`Rule::NoTwin`], or
    /// `None` for an opt-out that names a command.
    refused: Option<Failing<'static>>,
    /// The root verbs that [`Allow::root_verbs`] declares, or `None` for an
    /// opt-out.
    root_verbs: Option<&'static [&'static str]>,
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
            option: None,
            refused: None,
            root_verbs: None,
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
            option: None,
            refused: None,
            root_verbs: None,
            reason,
        }
    }

    /// Returns the opt-out from `rule` for the one option `option` of the
    /// command at `path`, for `reason`, so the other options of the command
    /// and the commands below it still follow the rule.
    ///
    /// Only [`Rule::RepeatsAndCommas`] and [`Rule::OneValuePerUse`] take
    /// it, because their problems are each one option's. `path` is as for
    /// [`Allow::new`], and `option` names the option as help does, such as
    /// `"--author"`, or `"-a"` without a long name. [`check_opt_outs`]
    /// names it as `shelf books list --author`, and [`check_filters`]
    /// fails on one whose command has no such option.
    ///
    /// ```
    /// use crumbkit_test::{Allow, Rule};
    ///
    /// const ALLOW: &[Allow] = &[
    ///     Allow::option(Rule::RepeatsAndCommas, "books list", "--author", "Authors' names hold commas."),
    ///     Allow::option(Rule::RepeatsAndCommas, "books list", "--title", "Titles hold commas."),
    /// ];
    /// ```
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, as
    /// [`Allow::new`] does, and for a rule other than the two filter rules:
    ///
    /// ```compile_fail
    /// use crumbkit_test::{Allow, Rule};
    ///
    /// const BAD: Allow = Allow::option(Rule::OneName, "books list", "--author", "Old scripts.");
    /// ```
    ///
    /// [`check_opt_outs`]: crate::check_opt_outs
    /// [`check_filters`]: crate::check_filters
    #[must_use]
    pub const fn option(
        rule: Rule,
        path: &'static str,
        option: &'static str,
        reason: &'static str,
    ) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        assert!(
            matches!(rule, Rule::RepeatsAndCommas | Rule::OneValuePerUse),
            "only the filter rules take an opt-out for one option"
        );
        Self {
            rule,
            path,
            words: None,
            option: Some(option),
            refused: None,
            root_verbs: None,
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
            option: None,
            refused: Some(refused),
            root_verbs: None,
            reason,
        }
    }

    /// Returns the declaration of `verbs`, the names of the commands without
    /// subcommands that sit directly under the root on purpose, such as
    /// `start` and `stop`, with the tool's `reason` for the whole list.
    ///
    /// It is the test-time form of `Guide::root_verbs`, for a tool that does
    /// not want root help to list its root verbs under their own heading. It
    /// is no opt-out: [`check_grammar`] passes each of `verbs` without an
    /// opt-out from [`Rule::RootLeaf`], fails a declared verb that names no
    /// visible command without subcommands directly under the root, and
    /// fails a tool that also declares root verbs on its guide.
    /// [`check_opt_outs`] lists the declaration with its reason among the
    /// customisations. Every other check ignores it.
    ///
    /// ```
    /// use crumbkit_test::Allow;
    ///
    /// const ALLOW: &[Allow] = &[Allow::root_verbs(
    ///     &["start", "stop"],
    ///     "People and scripts type `start` and `stop` as one word every day.",
    /// )];
    /// ```
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break, as
    /// [`Allow::new`] does.
    ///
    /// [`check_grammar`]: crate::check_grammar
    /// [`check_opt_outs`]: crate::check_opt_outs
    #[must_use]
    pub const fn root_verbs(verbs: &'static [&'static str], reason: &'static str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            rule: Rule::RootLeaf,
            path: "",
            words: None,
            option: None,
            refused: None,
            root_verbs: Some(verbs),
            reason,
        }
    }

    /// Returns the root verbs that [`Allow::root_verbs`] declares, or
    /// `None` for an opt-out.
    pub(crate) const fn declared_root_verbs(&self) -> Option<&'static [&'static str]> {
        self.root_verbs
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
    /// as `shelf books list`, the example's command line, such as
    /// `shelf books list --author 'Jane Austen'`, or the command line and
    /// the option, such as `shelf books list --author`.
    pub(crate) fn line(&self, root: &Command) -> String {
        if let Some(refused) = self.refused {
            return crate::guide::command_line(root.get_name(), refused.words());
        }
        let line = command_line(root, self.path);
        match (self.words, self.option) {
            (Some(words), _) => crate::guide::command_line(&line, words),
            (None, Some(option)) => format!("{line} {option}"),
            (None, None) => line,
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
            && self.option.is_none()
            && self.refused.is_none()
            && self.root_verbs.is_none()
            && (self.path.is_empty()
                || below
                    .strip_prefix(self.path)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with(' ')))
    }

    /// Reports whether the opt-out covers `rule` for the one option named
    /// `option` of the command at `below`.
    fn covers_option(&self, rule: Rule, below: &str, option: &str) -> bool {
        self.rule == rule && self.path == below && self.option == Some(option)
    }

    /// Reports whether the opt-out covers the one example `words` of the
    /// leaf at `below`.
    fn covers_example(&self, below: &str, words: &[&str]) -> bool {
        self.rule == Rule::Examples && self.path == below && self.words == Some(words)
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
