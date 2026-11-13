//! The codes whose recoveries the recovery check follows, and the report
//! of a code that no declared code spells.

use std::fmt::Write as _;

use crumbkit::{Code, Spelling};

use super::{Chain, Walk};

/// The codes of crumbkit's add-on crates: crumbkit-child's `codes::ALL` and
/// crumbkit-aliases's `codes::ALL`. A test compares this list with theirs.
pub(crate) const ADD_ON_CODES: &[&Code<'static>] = &[
    &Code::new("crumbkit", "child", "not_found"),
    &Code::new("crumbkit", "child", "not_started"),
    &Code::new("crumbkit", "aliases", "unreadable"),
];

/// Returns the codes of crumbkit and of its add-on crates: the one source
/// of the codes that the recovery check and the twin check treat as
/// raised by a crumbkit crate.
pub(crate) fn library_codes() -> impl Iterator<Item = &'static Code<'static>> {
    crumbkit::codes::ALL.iter().chain(ADD_ON_CODES).copied()
}

impl Walk<'_> {
    /// Returns whether the check follows the recovery of the spelled
    /// `code`: a code in `known`, or one of crumbkit's own codes or of its
    /// add-on crates' codes, which print with the tool's component
    /// (ADR-0079), such as crumbkit-child's `shelf::child::not_found`. Any
    /// other code in the tool's component, such as `shelf::cache::stale`,
    /// is unknown.
    pub(super) fn is_known(&self, code: &str) -> bool {
        self.known.iter().any(|known| known == code)
            || library_codes().any(|owned| self.spelling.spell(*owned) == code)
    }

    /// Writes `code`, which `label` failed with and which no declared code
    /// spells, to the report indented by `indent`. It adds a problem when
    /// the code's component is a declared component, and otherwise counts
    /// the code as skipped.
    pub(super) fn unknown(&mut self, chain: &Chain<'_>, label: &str, code: &str, indent: &str) {
        let _ = writeln!(
            self.report,
            "{indent}fails with {code}, which no declared code spells"
        );
        let declared = self
            .components
            .iter()
            .find(|(_, prefix)| code.len() > prefix.len() && code.starts_with(prefix.as_str()));
        match declared {
            Some((component, _)) => {
                self.problems.push(format!(
                    "{}: {label} fails with {code}, whose component {component} is a declared component, and no declared code spells it",
                    chain.start
                ));
            }
            None => self.skipped += 1,
        }
    }
}

/// Returns the text that `spelling` writes before the area of every code in
/// `component`, such as `shelf::`.
pub(in crate::recoveries) fn spelled_prefix(spelling: Spelling, component: &str) -> String {
    let code = Code::new(component, "area", "reason");
    format!("{}::", spelling.tool().code(code).component())
}

#[cfg(test)]
mod tests;
