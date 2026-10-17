//! The example check's rules for the lines of a `--json` example: one
//! document per line, or one indented document when the guide sets
//! `JsonLayout::Pretty` (ADR-0052).

use crumbkit::JsonLayout;
use crumbkit::help::Leaf;

/// Adds to `failures` each problem of `stdout`, the output of the `--json`
/// example `line` of `leaf` from a tool whose guide sets `layout`, as
/// [`check_examples`](super::check_examples) describes. A leaf without a document, or whose
/// document has no envelope, has none. Each document must hold its id
/// under its type key, which is `schema` unless the document's envelope
/// declares its own keys.
pub(super) fn check_lines(
    leaf: &Leaf,
    layout: Option<JsonLayout>,
    line: &str,
    stdout: &str,
    failures: &mut Vec<String>,
) {
    let Some(document) = leaf.get_document() else {
        return;
    };
    if !document.get_envelope().wraps() {
        return;
    }
    let key = document.get_envelope().type_key();
    let id = document.to_string();
    if !leaf.is_stream() && layout == Some(JsonLayout::Pretty) {
        check_indented(key, &id, line, stdout, failures);
        return;
    }
    let lines: Vec<&str> = stdout.lines().collect();
    if !leaf.is_stream() && lines.len() != 1 {
        failures.push(format!(
            "{line}: the command prints one document, so its --json output must be exactly one line, and the run printed {} lines",
            lines.len()
        ));
        return;
    }
    for (number, text) in (1..).zip(&lines) {
        let schema = serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .and_then(|value| value.get(key).cloned());
        if schema.as_ref().and_then(serde_json::Value::as_str) != Some(id.as_str()) {
            failures.push(format!(
                "{line}: line {number} of the --json output must be a JSON object whose {key} is {id}"
            ));
        }
    }
}

/// Adds to `failures` the problem of `stdout`, the output of the `--json`
/// example `line` of a leaf that prints one document whose `key` holds `id`,
/// from a tool whose guide sets `JsonLayout::Pretty`. The output must be one
/// JSON document indented by two spaces, as crumbkit prints it.
fn check_indented(key: &str, id: &str, line: &str, stdout: &str, failures: &mut Vec<String>) {
    let documents: Vec<_> = serde_json::Deserializer::from_str(stdout)
        .into_iter::<serde_json::Value>()
        .collect();
    let schema = match documents.as_slice() {
        [Ok(document)] => document.get(key).and_then(serde_json::Value::as_str),
        _ => None,
    };
    if schema != Some(id) {
        failures.push(format!(
            "{line}: the command prints one document, so its --json output must be exactly one JSON object whose {key} is {id}, and the run printed {} JSON documents",
            documents.iter().filter(|document| document.is_ok()).count()
        ));
    } else if !stdout
        .lines()
        .nth(1)
        .is_some_and(|second| second.starts_with("  \""))
    {
        let printed = if stdout.lines().count() == 1 {
            "on one line"
        } else {
            "with another indentation"
        };
        failures.push(format!(
            "{line}: the guide sets JsonLayout::Pretty, so the --json output must be one document indented by two spaces, and the run printed it {printed}"
        ));
    }
}
