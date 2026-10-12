//! The related diagnostics of a failure's miette diagnostic. With the
//! `miette` feature, the JSON failure document holds them under `related`,
//! so it carries what miette's graphical report shows. The text output
//! leaves them out.

use miette::{Diagnostic, Severity, SourceCode};
use serde::ser::{Serialize, SerializeMap, Serializer};

use super::label::{self, Label};

#[cfg(test)]
mod tests;

/// The most levels of related diagnostics that the document holds, counting
/// the failure's own related diagnostics as the first level. A diagnostic
/// can list itself or an ancestor as related, and reading such a chain
/// without a limit would never end. The related diagnostics below the last
/// level are left out.
const MOST_LEVELS: usize = 8;

/// One related diagnostic, read into the fields that the JSON document
/// holds.
pub(super) struct Related {
    /// The diagnostic's `Display`.
    message: String,
    /// The diagnostic's own code, which is free text that no codes check has
    /// seen, when it has one.
    code: Option<String>,
    /// The diagnostic's severity, `error` when it gives none, as miette
    /// treats it.
    severity: &'static str,
    /// The `Display` of each error below the diagnostic, in order.
    causes: Vec<String>,
    /// The diagnostic's labels.
    labels: Vec<Label>,
    /// The diagnostic's help, when it has some.
    help: Option<String>,
    /// The diagnostic's url, when it has one.
    url: Option<String>,
    /// The diagnostic's own related diagnostics.
    nested: Vec<Related>,
}

/// Reads the related diagnostics of `diagnostic`, whose labels read against
/// `source`, in the order the diagnostic gives them.
pub(super) fn read(diagnostic: &dyn Diagnostic, source: Option<&dyn SourceCode>) -> Vec<Related> {
    read_levels(diagnostic, source, MOST_LEVELS)
}

/// Reads the related diagnostics of `diagnostic` down to `levels` levels.
fn read_levels(
    diagnostic: &dyn Diagnostic,
    source: Option<&dyn SourceCode>,
    levels: usize,
) -> Vec<Related> {
    if levels == 0 {
        return Vec::new();
    }
    let Some(related) = diagnostic.related() else {
        return Vec::new();
    };
    related
        .map(|related| Related::read(related, source, levels - 1))
        .collect()
}

impl Related {
    /// Reads `diagnostic`, whose labels read against its own source code or
    /// else `parent_source`, and its related diagnostics down to `levels`
    /// more levels.
    fn read(
        diagnostic: &dyn Diagnostic,
        parent_source: Option<&dyn SourceCode>,
        levels: usize,
    ) -> Self {
        let source = diagnostic.source_code().or(parent_source);
        // miette's report lists the chain of `diagnostic_source()` as the
        // causes when there is one, and the `source()` chain otherwise.
        let first: Option<&dyn std::error::Error> = match diagnostic.diagnostic_source() {
            Some(below) => Some(below),
            None => diagnostic.source(),
        };
        let causes = std::iter::successors(first, |error| error.source())
            .map(ToString::to_string)
            .collect();
        let severity = match diagnostic.severity() {
            Some(Severity::Warning) => "warning",
            Some(Severity::Advice) => "advice",
            Some(Severity::Error) | None => "error",
        };
        Self {
            message: diagnostic.to_string(),
            code: diagnostic.code().map(|code| code.to_string()),
            severity,
            causes,
            labels: label::read(diagnostic, source),
            help: diagnostic.help().map(|help| help.to_string()),
            url: diagnostic.url().map(|url| url.to_string()),
            nested: read_levels(diagnostic, source, levels),
        }
    }
}

/// Serializes the related diagnostic as an object with `message`, `code`
/// when it has one, `severity`, `cause` when it has causes, joined with `: `
/// as the failure's are, `labels` when it has any, `help` and `url` when it
/// has them, and `related` when it has related diagnostics.
impl Serialize for Related {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("message", &self.message)?;
        if let Some(code) = &self.code {
            map.serialize_entry("code", code)?;
        }
        map.serialize_entry("severity", self.severity)?;
        if !self.causes.is_empty() {
            map.serialize_entry("cause", &self.causes.join(": "))?;
        }
        if !self.labels.is_empty() {
            map.serialize_entry("labels", &self.labels)?;
        }
        if let Some(help) = &self.help {
            map.serialize_entry("help", help)?;
        }
        if let Some(url) = &self.url {
            map.serialize_entry("url", url)?;
        }
        if !self.nested.is_empty() {
            map.serialize_entry("related", &self.nested)?;
        }
        map.end()
    }
}
