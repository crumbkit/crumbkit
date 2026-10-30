//! The tool's own facts about a run, which the usage log writes under the
//! line's `facts` field (ADR-0144).
//!
//! A tool keeps its facts in a type of its own that implements
//! [`Extension`], and changes them during the run with [`extend`]. When the
//! line is written, each extension writes its facts through [`Facts`], which
//! takes only counts, the time spent blocked waiting, and [`Label`]s that the tool
//! declares as constants, so the section holds no value that the caller
//! typed or that a server returned.

use std::any::Any;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use schemars::JsonSchema;
use serde::Serialize;

use super::recorder;

#[cfg(test)]
mod tests;

/// The name of one fact, such as `requests` or `status_5xx`.
///
/// A key is 1 to 32 bytes: a lowercase ASCII letter, then lowercase
/// letters, digits and `_`. A tool declares each key as a constant, so a
/// bad name fails to compile:
///
/// ```
/// use crumbkit_usage::Key;
///
/// const REQUESTS: Key = Key::new("requests");
/// ```
///
/// ```compile_fail
/// use crumbkit_usage::Key;
///
/// const BAD: Key = Key::new("Requests sent");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key(&'static str);

impl Key {
    /// Returns the key `name`.
    ///
    /// # Panics
    ///
    /// Panics when `name` is not a key, which is a compile error in a
    /// constant.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        assert!(
            is_key(name),
            "a usage key is 1 to 32 bytes of a-z, 0-9 and _, starting with a letter"
        );
        Self(name)
    }
}

/// One value from the tool's own fixed vocabulary, such as a route template
/// `/books/{id}`, a status class `5xx` or a kind of target `staging`.
///
/// A label is 1 to 64 bytes of ASCII letters, digits, `_`, `-`, `.`, `/`,
/// `:`, `{` and `}`, so it holds no space, query string or `@`. Its text is
/// a `&'static str`, and a tool declares each label as a constant, so a
/// value that the caller typed or a server returned cannot become a label
/// without leaking it into a `'static` string on purpose.
///
/// ```
/// use crumbkit_usage::Label;
///
/// const SHOW: Label = Label::new("/books/{id}");
/// ```
///
/// ```compile_fail
/// use crumbkit_usage::Label;
///
/// const BAD: Label = Label::new("/books/42?key=abc");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Label(&'static str);

impl Label {
    /// Returns the label `text`.
    ///
    /// # Panics
    ///
    /// Panics when `text` is not a label, which is a compile error in a
    /// constant.
    #[must_use]
    pub const fn new(text: &'static str) -> Self {
        assert!(
            is_label(text),
            "a usage label is 1 to 64 bytes of ASCII letters, digits, braces and _-./:"
        );
        Self(text)
    }
}

/// A tool's own facts about one run, which the usage log writes under the
/// line's `facts` field.
///
/// The tool's type holds whatever it needs during the run, and [`write`]
/// turns it into facts when the line is written. [`extend`] creates the
/// value with `Default` on its first use in a run.
///
/// `write` runs as the run ends, from the guard's drop or from the thread
/// that answers SIGINT and SIGTERM, which nothing bounds, so it must be
/// short and must not wait on other threads. A `write` that panics costs
/// the line its facts. The panic message is still printed to standard
/// error, and the line is still written with the run's exit code only when
/// panics unwind: under `panic = "abort"`, or with a panic hook that ends
/// the process, the panic ends the run.
///
/// ```
/// use std::time::Duration;
/// use crumbkit_usage::{Extension, Facts, Key, Label};
///
/// const REQUESTS: Key = Key::new("requests");
/// const ROUTE: Key = Key::new("route");
///
/// #[derive(Default)]
/// struct Requests {
///     sent: u64,
///     routes: Vec<Label>,
///     /// The wall-clock time the run spent blocked on the server. The tool
///     /// sends one request at a time, so it adds each request's time.
///     blocked: Duration,
/// }
///
/// impl Extension for Requests {
///     fn write(&self, facts: &mut Facts) {
///         facts.count(REQUESTS, self.sent).waited(self.blocked);
///         for route in &self.routes {
///             facts.label(ROUTE, *route);
///         }
///     }
/// }
/// ```
///
/// [`write`]: Extension::write
pub trait Extension: Default + Send + 'static {
    /// Writes this run's facts into `facts`.
    fn write(&self, facts: &mut Facts);
}

/// Changes this run's value of the extension `E`, creating it with
/// `Default` on its first use.
///
/// ```
/// # use crumbkit_usage::{Extension, Facts, Key};
/// # const REQUESTS: Key = Key::new("requests");
/// # #[derive(Default)]
/// # struct Requests { sent: u64 }
/// # impl Extension for Requests {
/// #     fn write(&self, facts: &mut Facts) {
/// #         facts.count(REQUESTS, self.sent);
/// #     }
/// # }
/// crumbkit_usage::extend(|requests: &mut Requests| requests.sent += 1);
/// ```
///
/// Without a started log, `change` does not run, so a run without the log
/// pays for nothing but the check. A run may keep several extensions, such
/// as the tool's own and one of a library that it uses, and their facts
/// share the one section.
///
/// `change` runs while the log holds its lock, so it must be short and must
/// not wait on other threads. On SIGINT or SIGTERM the log waits at most
/// 500 ms for its lock, and then lets the signal end the process without
/// the run's line. A call to `extend`, or to any of crumbkit's reports, that
/// `change` makes on the same thread is skipped without a word, as every
/// other usage-log failure is, and `change` itself still applies. A
/// `change` that panics leaves the run's line without facts, and the panic
/// goes on to the caller. The section counts towards the line's limit of
/// 1 KiB, and a line that would be longer is written without its `facts`
/// field.
pub fn extend<E: Extension>(change: impl FnOnce(&mut E)) {
    recorder::report(|recorder| recorder.extensions.change(change));
}

/// The facts that a run's extensions write. A tool receives it in
/// [`Extension::write`] and cannot build one, so every fact passes through
/// these methods.
#[derive(Debug)]
pub struct Facts(pub(super) Written);

impl Facts {
    /// Returns facts that hold nothing yet, which crumbkit-usage passes to
    /// each extension in turn.
    pub(crate) fn new() -> Self {
        Self(Written::default())
    }

    /// Adds `count` to the fact `key`. A count that would pass `u64::MAX`
    /// stays there.
    pub fn count(&mut self, key: Key, count: u64) -> &mut Self {
        let total = self.0.counts.entry(key.0).or_default();
        *total = total.saturating_add(count);
        self
    }

    /// Adds `label` to the labels of `key`, once, after the labels it
    /// already holds.
    pub fn label(&mut self, key: Key, label: Label) -> &mut Self {
        let labels = self.0.labels.entry(key.0).or_default();
        if !labels.contains(&label.0) {
            labels.push(label.0);
        }
        self
    }

    /// Adds `time` to the wall-clock time that the run spent blocked,
    /// waiting on another system such as a server, a database or a child
    /// process. `self usage` compares it with the run's wall time (ADR-0145).
    ///
    /// Overlapping waits count once. A tool that sends requests one at a
    /// time adds the time of each, and a tool that sends several at once
    /// adds the time from the first one's start to the last one's end, not
    /// the sum of their times. Several calls add, so each call passes a
    /// stretch of time that no other call covers.
    pub fn waited(&mut self, time: Duration) -> &mut Self {
        let ms = u64::try_from(time.as_millis()).unwrap_or(u64::MAX);
        self.0.waited_ms = self.0.waited_ms.saturating_add(ms);
        self
    }
}

/// The tool's own facts about a run: counts, labels from the tool's fixed
/// vocabulary, and the time the run spent waiting on another system.
#[derive(Debug, Default, Serialize, JsonSchema)]
#[schemars(rename = "Facts")]
pub(super) struct Written {
    /// Counts by name, such as requests or retries. A count of 0 is left
    /// out.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schemars(extend(
        "propertyNames" = { "pattern": "^[a-z][a-z0-9_]{0,31}$" },
        "additionalProperties" = { "type": "integer", "format": "uint64", "minimum": 1 }
    ))]
    counts: BTreeMap<&'static str, u64>,
    /// Labels by name, such as the route templates that the run requested,
    /// each once, in the order the tool wrote them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schemars(with = "BTreeMap<String, Vec<LabelText>>")]
    #[schemars(extend(
        "propertyNames" = { "pattern": "^[a-z][a-z0-9_]{0,31}$" },
        "additionalProperties" = {
            "type": "array",
            "items": { "$ref": "#/$defs/Label" },
            "uniqueItems": true
        }
    ))]
    labels: BTreeMap<&'static str, Vec<&'static str>>,
    /// The wall-clock milliseconds that the run spent blocked, waiting on
    /// another system such as a server, with overlapping waits counted
    /// once. It is present only when it is more than 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    #[schemars(extend("minimum" = 1))]
    waited_ms: u64,
}

impl Written {
    /// Returns whether the facts hold nothing to write: no count above 0, no
    /// label and no time waiting.
    fn is_empty(&self) -> bool {
        self.counts.values().all(|count| *count == 0)
            && self.labels.is_empty()
            && self.waited_ms == 0
    }
}

/// One label from the tool's fixed vocabulary, such as a route template.
#[derive(JsonSchema)]
#[schemars(rename = "Label", extend("pattern" = "^[A-Za-z0-9_./:{}-]{1,64}$"))]
#[expect(
    dead_code,
    reason = "the type exists only to describe a label in the schema"
)]
struct LabelText(String);

/// One extension of a run, stored without its type.
trait Stored: Send {
    /// Returns the extension, so [`Extensions::change`] can find its type.
    fn as_any(&mut self) -> &mut dyn Any;

    /// Writes the extension's facts, as [`Extension::write`] does.
    fn write_to(&self, facts: &mut Facts);
}

impl<E: Extension> Stored for E {
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn write_to(&self, facts: &mut Facts) {
        self.write(facts);
    }
}

/// The extensions of a run, in the order of their first use.
#[derive(Default)]
pub(super) struct Extensions {
    /// The extensions, each stored without its type.
    stored: Vec<Box<dyn Stored>>,
    /// Whether a change panicked, which may have left an extension half
    /// changed, so the run writes no facts.
    broken: bool,
}

impl Extensions {
    /// Applies `change` to this run's `E`, creating it first when the run
    /// has none.
    pub(super) fn change<E: Extension>(&mut self, change: impl FnOnce(&mut E)) {
        for stored in &mut self.stored {
            if let Some(extension) = stored.as_any().downcast_mut::<E>() {
                change(extension);
                return;
            }
        }
        let mut extension = E::default();
        change(&mut extension);
        self.stored.push(Box::new(extension));
    }

    /// Marks the extensions broken after a change panicked, so the run
    /// writes no facts.
    pub(super) fn broke(&mut self) {
        self.broken = true;
    }

    /// Returns the facts that every extension writes, or `None` when they
    /// hold nothing, a change panicked, or an extension's `write` panics.
    ///
    /// The line is written as the run ends, from the guard's drop or the
    /// signal thread, where a panic that went on would change the exit code
    /// or leave a signal ignored, so a panicking `write` costs the run only
    /// its facts (ADR-0029).
    pub(super) fn written(&self) -> Option<Written> {
        if self.broken {
            return None;
        }
        let mut facts = Facts::new();
        catch_unwind(AssertUnwindSafe(|| {
            for stored in &self.stored {
                stored.write_to(&mut facts);
            }
        }))
        .ok()?;
        facts.0.counts.retain(|_, count| *count > 0);
        (!facts.0.is_empty()).then_some(facts.0)
    }
}

/// Returns whether `name` is a [`Key`]: 1 to 32 bytes, a lowercase ASCII
/// letter followed by lowercase letters, digits and `_`.
const fn is_key(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || bytes.len() > 32 || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    let mut index = 1;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_') {
            return false;
        }
        index += 1;
    }
    true
}

/// Returns whether `text` is a [`Label`]: 1 to 64 bytes of ASCII letters,
/// digits and `_-./:{}`.
const fn is_label(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if !(byte.is_ascii_alphanumeric()
            || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':' | b'{' | b'}'))
        {
            return false;
        }
        index += 1;
    }
    true
}

/// Returns whether `value` is 0, so serde leaves out a time not spent.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes the field by reference"
)]
const fn is_zero(value: &u64) -> bool {
    *value == 0
}
