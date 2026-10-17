//! The codes whose recoveries the recovery check follows, and the report
//! of a code that no declared code spells.

use std::fmt::Write as _;

use super::{Chain, Walk};

impl Walk<'_> {
    /// Returns whether the check follows the recovery of the spelled
    /// `code`: a code in `known`, or a code that one of crumbkit's add-on
    /// crates raises, such as crumbkit-child's `shelf::child::failed`.
    /// crumbkit's own codes print with the tool's component (ADR-0079), so
    /// the check takes a code in that component for an add-on crate's code
    /// when no declared code of the component has its area. A code in an
    /// area that the tool declares, such as `shelf::books::lost` beside a
    /// declared `shelf::books::not_found`, stays unknown.
    pub(super) fn is_known(&self, code: &str) -> bool {
        if self.known.iter().any(|known| known == code) {
            return true;
        }
        let own = self.spelling.tool().code(&crumbkit::codes::CLI_USAGE);
        let Some((component, area, _)) = self.spelling.split(code) else {
            return false;
        };
        component == own.component()
            && !self.declared.iter().any(|declared| {
                self.spelling
                    .split(declared)
                    .is_some_and(|(read, declared_area, _)| {
                        read == component && declared_area == area
                    })
            })
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
