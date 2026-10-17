//! The codes whose recoveries the recovery check follows, and the report
//! of a code that no declared code spells.

use std::fmt::Write as _;

use crumbkit::Code;

use super::{Chain, Walk};

/// The codes of crumbkit's add-on crates: crumbkit-child's `codes::ALL` and
/// crumbkit-aliases's `codes::ALL`. A test compares this list with theirs.
pub(crate) const ADD_ON_CODES: &[&Code] = &[
    &Code::new("crumbkit", "child", "no_json"),
    &Code::new("crumbkit", "child", "not_found"),
    &Code::new("crumbkit", "child", "not_started"),
    &Code::new("crumbkit", "child", "bad_json"),
    &Code::new("crumbkit", "child", "failed"),
    &Code::new("crumbkit", "aliases", "unreadable"),
];

impl Walk<'_> {
    /// Returns whether the check follows the recovery of the spelled
    /// `code`: a code in `known`, or one of crumbkit's own codes or of its
    /// add-on crates' codes, which print with the tool's component
    /// (ADR-0079), such as crumbkit-child's `shelf::child::failed`. Any
    /// other code in the tool's component, such as `shelf::cache::stale`,
    /// is unknown.
    pub(super) fn is_known(&self, code: &str) -> bool {
        self.known.iter().any(|known| known == code)
            || crumbkit::codes::ALL
                .iter()
                .chain(ADD_ON_CODES)
                .any(|&owned| self.spelling.spell(owned) == code)
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
        match self.spelling.split(code) {
            Some((component, _, _)) if self.components.iter().any(|known| known == component) => {
                self.problems.push(format!(
                    "{}: {label} fails with {code}, whose component {component} is a declared component, and no declared code spells it",
                    chain.start
                ));
            }
            _ => self.skipped += 1,
        }
    }
}
