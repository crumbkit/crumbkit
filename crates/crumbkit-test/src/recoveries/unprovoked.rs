//! The declared codes that no command line in a test can provoke.

use crumbkit::{Code, Spelling};

use crate::allow::is_reason;

/// What an [`Unprovoked`] entry lists: one code, or every declared code of
/// one area.
#[derive(Clone, Copy, Debug)]
enum Listed<'a> {
    /// One declared code.
    Code(Code<'a>),
    /// Every declared code with this component and area.
    Area {
        /// The component that owns the codes, such as `tool`.
        component: &'a str,
        /// The area of the codes, such as `api`.
        area: &'a str,
    },
}

/// A declared code, or every declared code of one area, that no command
/// line in a test can provoke, with the reason, such as a failure that needs
/// a full disk. [`check_recoveries`](crate::check_recoveries)
/// starts no chain for it, and [`check_twins`](crate::check_twins) needs no
/// twin for it, so a tool passes the same list to both.
#[derive(Clone, Copy, Debug)]
pub struct Unprovoked<'a> {
    /// What the entry lists.
    listed: Listed<'a>,
    /// Why no test can provoke it, in one line.
    reason: &'a str,
}

impl<'a> Unprovoked<'a> {
    /// Returns the entry that lists `code` as unprovoked for `reason`.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A
    /// `const` that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit::Code;
    /// use crumbkit_test::Unprovoked;
    ///
    /// const GONE: Code<'static> = Code::new("shelf", "books", "removed_file");
    /// const BAD: Unprovoked<'_> = Unprovoked::new(GONE, "");
    /// ```
    #[must_use]
    pub const fn new(code: Code<'a>, reason: &'a str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            listed: Listed::Code(code),
            reason,
        }
    }

    /// Returns the entry that lists every declared code of `area` in
    /// `component` as unprovoked for `reason`, such as the codes of a server
    /// API that the binary never prints. Both checks accept it wherever
    /// they accept [`Unprovoked::new`]. A code declared in the area later
    /// is excused too, and the recovery report names it.
    ///
    /// An entry that covers no declared code, that covers codes which are all
    /// provoked or refused, or that a code entry in the same area repeats,
    /// fails the check.
    ///
    /// # Panics
    ///
    /// Panics unless `reason` is non-empty and holds no line break. A
    /// `const` that calls it with such a reason fails to compile:
    ///
    /// ```compile_fail
    /// use crumbkit_test::Unprovoked;
    ///
    /// const BAD: Unprovoked<'_> = Unprovoked::area("shelf", "api", "");
    /// ```
    #[must_use]
    pub const fn area(component: &'a str, area: &'a str, reason: &'a str) -> Self {
        assert!(is_reason(reason), "a reason is one non-empty line");
        Self {
            listed: Listed::Area { component, area },
            reason,
        }
    }

    /// Returns the listed code, or `None` for an area entry.
    fn code(&self) -> Option<Code<'a>> {
        match self.listed {
            Listed::Code(code) => Some(code),
            Listed::Area { .. } => None,
        }
    }

    /// Reports whether the entry lists `code`.
    pub(crate) fn covers(&self, code: Code<'_>) -> bool {
        match self.listed {
            Listed::Code(listed) => listed == code,
            Listed::Area { component, area } => {
                code.component() == component && code.area() == area
            }
        }
    }

    /// Returns why no test can provoke the listed codes.
    pub(crate) const fn reason(&self) -> &'a str {
        self.reason
    }

    /// Returns the entry as the report spells it: the code, or the area
    /// followed by `::*`.
    fn spelled(&self, spelling: Spelling) -> String {
        match self.listed {
            Listed::Code(code) => spelling.spell(code),
            Listed::Area { component, area } => {
                let text = spelling.spell(Code::new(component, area, "x"));
                format!("{}*", text.strip_suffix('x').unwrap_or(&text))
            }
        }
    }
}

/// Reports whether any of `entries` lists `code`.
pub(crate) fn lists(entries: &[Unprovoked<'_>], code: Code<'_>) -> bool {
    entries.iter().any(|entry| entry.covers(code))
}

/// Returns the problems of `entries` against the `declared` codes, and
/// appends each entry to `report` when it is `Some`, an area entry with the
/// declared codes it excuses below it, which are those that are not met. `setting` names the setting that
/// holds the entries, `by` says what meets a code, such as `a start provokes`,
/// and `met` reports whether a code is met.
pub(crate) fn problems(
    entries: &[Unprovoked<'_>],
    declared: &[Code<'_>],
    spelling: Spelling,
    setting: &str,
    by: &str,
    met: impl Fn(Code<'_>) -> bool,
    mut report: Option<&mut String>,
) -> Vec<String> {
    use std::fmt::Write as _;
    let mut problems = Vec::new();
    for entry in entries {
        let text = entry.spelled(spelling);
        let covered: Vec<Code<'_>> = declared
            .iter()
            .copied()
            .filter(|&code| entry.covers(code))
            .collect();
        match entry.code() {
            Some(code) => {
                if covered.is_empty() {
                    problems.push(format!(
                        "{text}: {setting} lists the code, and it is not a declared code"
                    ));
                } else if let Some(area) = entries
                    .iter()
                    .find(|other| other.code().is_none() && other.covers(code))
                {
                    problems.push(format!(
                        "{text}: {setting} lists the code, and it repeats the area entry {}",
                        area.spelled(spelling)
                    ));
                } else if met(code) {
                    problems.push(format!("{text}: {setting} lists the code, and {by} it"));
                }
            }
            None if covered.is_empty() => problems.push(format!(
                "{text}: {setting} lists the area, and it covers no declared code"
            )),
            None if covered.iter().all(|&code| met(code)) => problems.push(format!(
                "{text}: {setting} lists the area, and {by} every declared code in it"
            )),
            None => {}
        }
        if let Some(report) = report.as_deref_mut() {
            let _ = writeln!(report, "  {text}: {}", entry.reason());
            if entry.code().is_none() {
                for &code in covered.iter().filter(|&&code| !met(code)) {
                    let _ = writeln!(report, "    {}", spelling.spell(code));
                }
            }
        }
    }
    problems
}
