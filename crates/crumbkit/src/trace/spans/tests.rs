//! Tests for the spans of `parse` and `finish`, through a small subscriber
//! that keeps every span, field and event it sees.

use std::fmt;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::Interest;
use tracing::{Event, Level, Metadata, Subscriber};
use tracing_core::span::Current;

use super::record_ids;
use crate::help::Guide;
use crate::{
    Batch, Code, Document, Error, Failure, Output, Recovery, Row, Sink, Spelling, finish_to,
    parse_to,
};

const NOT_FOUND: Code = Code::new("solar", "planets", "not_found");

/// A word that no span may hold, because the caller typed it.
const PLANTED: &str = "s3cr3t";

/// A span that the subscriber saw, with every field recorded on it.
struct Seen {
    metadata: &'static Metadata<'static>,
    fields: Vec<(&'static str, String)>,
}

/// What the subscriber saw: its spans in the order they opened, the spans
/// entered now, and how many events arrived.
#[derive(Default)]
struct Sightings {
    spans: Vec<Seen>,
    entered: Vec<Id>,
    events: usize,
}

impl Sightings {
    /// Returns the names of the spans in the order they opened.
    fn names(&self) -> Vec<&'static str> {
        self.spans.iter().map(|span| span.metadata.name()).collect()
    }

    /// Returns the fields recorded on the span called `name`.
    fn fields(&self, name: &str) -> &[(&'static str, String)] {
        let span = self.spans.iter().find(|span| span.metadata.name() == name);
        &span.unwrap().fields
    }
}

/// A subscriber that keeps what it sees in shared memory, so the test reads
/// it after the subscriber is gone. With `quiet`, it disables every span
/// whose target is `crumbkit`.
#[derive(Clone, Default)]
struct Collector {
    seen: Arc<Mutex<Sightings>>,
    quiet: bool,
}

/// Writes each field it visits into a list as text.
struct Fields<'a>(&'a mut Vec<(&'static str, String)>);

impl Visit for Fields<'_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name(), value.to_owned()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.0.push((field.name(), format!("{value:?}")));
    }
}

/// Returns the position in [`Sightings::spans`] of the span `id`.
fn index(id: &Id) -> usize {
    usize::try_from(id.into_u64()).unwrap() - 1
}

impl Subscriber for Collector {
    // Each test installs its own collector, so the collector asks `enabled`
    // on every call and never lets tracing cache one test's answer.
    fn register_callsite(&self, _: &'static Metadata<'static>) -> Interest {
        Interest::sometimes()
    }

    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        !(self.quiet && metadata.target() == "crumbkit")
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        let mut sightings = self.seen.lock().unwrap();
        let mut fields = Vec::new();
        span.record(&mut Fields(&mut fields));
        sightings.spans.push(Seen {
            metadata: span.metadata(),
            fields,
        });
        Id::from_u64(u64::try_from(sightings.spans.len()).unwrap())
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut sightings = self.seen.lock().unwrap();
        values.record(&mut Fields(&mut sightings.spans[index(span)].fields));
    }

    fn record_follows_from(&self, _: &Id, _: &Id) {}

    fn event(&self, _: &Event<'_>) {
        self.seen.lock().unwrap().events += 1;
    }

    fn enter(&self, span: &Id) {
        self.seen.lock().unwrap().entered.push(span.clone());
    }

    fn exit(&self, _: &Id) {
        self.seen.lock().unwrap().entered.pop();
    }

    fn current_span(&self) -> Current {
        let sightings = self.seen.lock().unwrap();
        match sightings.entered.last() {
            Some(id) => Current::new(id.clone(), sightings.spans[index(id)].metadata),
            None => Current::none(),
        }
    }
}

/// Returns what a subscriber for this thread saw while `run` ran.
fn watch(run: impl FnOnce()) -> Sightings {
    watch_with(&Collector::default(), run)
}

/// Returns what `collector`, as the subscriber for this thread, saw while
/// `run` ran.
fn watch_with(collector: &Collector, run: impl FnOnce()) -> Sightings {
    tracing::subscriber::with_default(collector.clone(), run);
    std::mem::take(&mut *collector.seen.lock().unwrap())
}

/// A row with one column.
#[derive(Serialize, JsonSchema)]
struct Planet {
    name: &'static str,
}

impl Row for Planet {
    const COLUMNS: &'static [&'static str] = &["name"];
}

const PLANETS: Document<Vec<Planet>> = Document::new("solar", "planets-show", 1, "The test rows.");

/// A failure that names a planet the solar system lacks.
#[derive(Debug)]
struct Missing;

impl fmt::Display for Missing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the solar system has no such planet")
    }
}

impl std::error::Error for Missing {}

impl Failure for Missing {
    fn code(&self) -> &'static Code {
        &NOT_FOUND
    }

    fn recovery(&self) -> Recovery {
        Recovery::Hint("name a planet of the solar system".to_owned())
    }
}

/// A tool with a `show` command that takes many names.
#[derive(Parser)]
#[command(name = "solar", version = "1.2.3")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    output: Output,
}

#[derive(Subcommand)]
enum Command {
    Planets {
        #[command(subcommand)]
        command: Planets,
    },
}

#[derive(Subcommand)]
enum Planets {
    Show {
        #[arg(required = true, value_name = "NAME")]
        names: Vec<String>,
    },
}

/// Parses `args` into memory, and returns the parsed tool's output flags
/// with the tool's command, or the exit code of a run that parsing ended.
fn parse(args: &[&str]) -> Result<Output, ExitCode> {
    let (tool, command) = parse_to::<Tool, _, _, _, _>(
        &mut Sink::new(Vec::new(), Vec::new()),
        Tool::command(),
        &Guide::new(),
        Spelling::COLONS,
        args,
    )?;
    let mut output = tool.output;
    output.set_command(command);
    Ok(output)
}

/// Parses `args` and finishes `result` into memory, as a tool's `main`
/// does.
fn run<T: crate::Rows, E: crate::AsFailure>(
    args: &[&str],
    document: Document<T>,
    result: Result<T, E>,
) {
    if let Ok(output) = parse(args) {
        let sink = &mut Sink::new(Vec::new(), Vec::new());
        finish_to(sink, &output, Spelling::COLONS, document, result);
    }
}

#[test]
fn a_run_opens_the_parse_span_and_then_the_finish_span_at_info_under_crumbkit() {
    let seen = watch(|| {
        run(
            &["solar", "planets", "show", PLANTED],
            PLANETS,
            Ok::<_, Error>(Vec::new()),
        );
    });

    assert_eq!(seen.names(), ["crumbkit.parse", "crumbkit.finish"]);
    for span in &seen.spans {
        assert_eq!(span.metadata.target(), "crumbkit");
        assert_eq!(*span.metadata.level(), Level::INFO);
    }
    assert_eq!(
        seen.fields("crumbkit.parse"),
        [("crumbkit.command", "planets show".to_owned())]
    );
    assert_eq!(
        seen.fields("crumbkit.finish"),
        [("process.exit.code", "0".to_owned())]
    );
}

#[test]
fn no_span_or_event_holds_a_value_that_the_caller_typed() {
    let seen = watch(|| {
        run(
            &["solar", "planets", "show", PLANTED],
            PLANETS,
            Err(Missing),
        );
        run(
            &["solar", "planets", "show", "--fields", PLANTED],
            PLANETS,
            Err(Missing),
        );
        run(&["solar", PLANTED], PLANETS, Err(Missing));
    });

    assert_eq!(seen.events, 0);
    for span in &seen.spans {
        for (name, value) in &span.fields {
            assert!(!value.contains(PLANTED), "{name} holds {value}");
        }
    }
}

#[test]
fn a_run_that_parse_ends_records_its_exit_code_and_a_refusal_s_code_on_the_parse_span() {
    let help = watch(|| {
        let _ = parse(&["solar", "planets", "show", "--help"]);
    });
    let refusal = watch(|| {
        let _ = parse(&["solar", "moons"]);
    });

    assert_eq!(help.names(), ["crumbkit.parse"]);
    assert_eq!(
        help.fields("crumbkit.parse"),
        [
            ("crumbkit.command", "planets show".to_owned()),
            ("process.exit.code", "0".to_owned())
        ]
    );
    assert_eq!(
        refusal.fields("crumbkit.parse"),
        [
            ("error.type", "crumbkit::cli::usage".to_owned()),
            ("process.exit.code", "2".to_owned())
        ]
    );
}

#[test]
fn a_failure_records_its_code_in_the_tool_s_spelling_and_exit_code_1_on_the_finish_span() {
    let seen = watch(|| {
        run(
            &["solar", "planets", "show", "Vulcan"],
            PLANETS,
            Err(Missing),
        );
    });

    assert_eq!(
        seen.fields("crumbkit.finish"),
        [
            ("error.type", "solar::planets::not_found".to_owned()),
            ("process.exit.code", "1".to_owned())
        ]
    );
}

#[test]
fn a_batch_records_only_its_first_failure_s_code() {
    let batch = Batch::new(
        vec![Planet { name: "Saturn" }],
        vec![Error::from(Missing), Error::from(Missing)],
    );
    let document = Document::new("solar", "planets-show", 1, "The test rows.");

    let seen = watch(|| {
        run(
            &["solar", "planets", "show", "Saturn"],
            document,
            Ok::<_, Error>(batch),
        );
    });

    assert_eq!(
        seen.fields("crumbkit.finish"),
        [
            ("error.type", "solar::planets::not_found".to_owned()),
            ("process.exit.code", "1".to_owned())
        ]
    );
}

#[test]
fn a_span_records_the_trace_id_and_the_run_s_id_that_the_usage_log_gives_it() {
    let ids = Some((
        "4bf92f3577b34da6a3ce929d0e0e4736".to_owned(),
        "2222222222222222".to_owned(),
    ));

    let seen = watch(|| record_ids(&super::finish(), ids));

    assert_eq!(
        seen.fields("crumbkit.finish"),
        [
            ("trace_id", "4bf92f3577b34da6a3ce929d0e0e4736".to_owned()),
            ("crumbkit.run", "2222222222222222".to_owned())
        ]
    );
}

#[test]
fn a_subscriber_that_disables_crumbkit_s_spans_sees_no_crumbkit_field_on_the_tool_s_span() {
    let collector = Collector {
        quiet: true,
        ..Collector::default()
    };

    let seen = watch_with(&collector, || {
        let span = tracing::info_span!(
            "solar",
            "crumbkit.command" = tracing::field::Empty,
            "error.type" = tracing::field::Empty,
        );
        let _entered = span.enter();
        run(
            &["solar", "planets", "show", "Vulcan"],
            PLANETS,
            Err(Missing),
        );
        let _ = parse(&["solar", "moons"]);
    });

    assert_eq!(seen.names(), ["solar"]);
    assert_eq!(seen.fields("solar"), []);
}
