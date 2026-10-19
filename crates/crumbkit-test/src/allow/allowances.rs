//! The record of which opt-outs of a check excused a problem, which finds
//! the stale ones.

use clap::Command;

use super::{Allow, Rule, words};
use crate::tree;

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

    /// Reports whether an opt-out covers `rule` for the option named
    /// `option` of the command at `below`: an opt-out for the command or one
    /// above it, or one for that option alone. It records each opt-out that
    /// does as used.
    pub(crate) fn excuses_option(&mut self, rule: Rule, below: &str, option: &str) -> bool {
        let mut excused = false;
        for (allow, used) in self.allow.iter().zip(&mut self.used) {
            if self.rules.contains(&rule)
                && (allow.covers(rule, below) || allow.covers_option(rule, below, option))
            {
                *used = true;
                excused = true;
            }
        }
        excused
    }

    /// Reports whether an example opt-out covers the example `words` of the
    /// leaf at `below`, and records each one that does as used.
    pub(crate) fn excuses_example(&mut self, below: &str, words: &[&str]) -> bool {
        let mut excused = false;
        for (allow, used) in self.allow.iter().zip(&mut self.used) {
            if self.rules.contains(&Rule::Examples) && allow.covers_example(below, words) {
                *used = true;
                excused = true;
            }
        }
        excused
    }

    /// Adds a problem to `problems` for each opt-out of the check's rules
    /// that names no command below `root`, names no option of its command,
    /// or excused no problem.
    pub(crate) fn finish(self, root: &Command, problems: &mut Vec<String>) {
        for (allow, used) in self.allow.iter().zip(self.used) {
            if !self.rules.contains(&allow.rule) {
                continue;
            }
            let line = allow.line(root);
            let id = allow.rule.id();
            let Some(command) = tree::find(root, words(allow.path)) else {
                problems.push(format!(
                    "the opt-out from `{id}` for `{line}` names no command"
                ));
                continue;
            };
            if allow.option.is_some_and(|option| {
                !command
                    .get_arguments()
                    .any(|arg| tree::argument_name(arg) == option)
            }) {
                problems.push(format!(
                    "the opt-out from `{id}` for `{line}` names no option of the command"
                ));
            } else if !used && allow.words.is_some() {
                problems.push(format!(
                    "the opt-out from `{id}` for `{line}` names no example that the check runs, so it is stale"
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
