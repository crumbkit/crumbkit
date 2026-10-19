//! Tests for the observer: how it is installed, and what `parse`, `finish`,
//! a stream and the public reports tell it.

use std::fmt;
use std::process::ExitCode;
use std::sync::OnceLock;

use clap::{CommandFactory as _, Parser, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;

use super::testing::{DOTS, give_child_env, watch};
use super::{Observer, SLOTS, Slots, child_env_in, handles_interrupt_in, installed, observe_in};
use crate::help::{Exit, Guide};
use crate::{
    Code, Document, Error, Failure, Listing, Output, Recovery, Row, Sink, Stream, finish_to,
    parse_to, stream_to,
};

/// An observer that names itself when it hears `aliased` and gives fixed
/// answers to the three questions.
struct Named {
    /// The name that `aliased` adds to this thread's list.
    name: &'static str,
    /// Whether it handles SIGINT.
    interrupt: bool,
    /// The child variables that it gives.
    env: &'static [(&'static str, &'static str)],
    /// The trace id and run id that it gives, if any.
    ids: Option<(&'static str, &'static str)>,
}

thread_local! {
    /// The names of the observers that heard `aliased` on this thread.
    static HEARD: std::cell::RefCell<Vec<&'static str>> = const { std::cell::RefCell::new(Vec::new()) };
}

impl Observer for Named {
    fn aliased(&self) {
        HEARD.with_borrow_mut(|heard| heard.push(self.name));
    }

    fn handles_interrupt(&self) -> bool {
        self.interrupt
    }

    fn child_env(&self) -> Vec<(&'static str, String)> {
        self.env
            .iter()
            .map(|&(name, value)| (name, value.to_owned()))
            .collect()
    }

    fn trace_ids(&self) -> Option<super::TraceIds> {
        self.ids
            .map(|(trace, run)| super::TraceIds::new(trace.to_owned(), run.to_owned()))
    }
}

/// Returns an observer named `name` that handles no interrupt and gives no
/// variables and no ids.
const fn plain(name: &'static str) -> Named {
    Named {
        name,
        interrupt: false,
        env: &[],
        ids: None,
    }
}

static FIRST: Named = plain("first");
static SECOND: Named = plain("second");
static THIRD: Named = plain("third");
static FOURTH: Named = plain("fourth");
static FIFTH: Named = plain("fifth");

/// Returns empty observer slots, as a process starts with.
fn empty_slots() -> Slots {
    [const { OnceLock::new() }; SLOTS]
}

/// Returns the names of the observers in `slots` that hear `aliased`, in
/// the order they hear it.
fn heard(slots: &Slots) -> Vec<&'static str> {
    HEARD.with_borrow_mut(Vec::clear);
    for observer in installed(slots) {
        observer.aliased();
    }
    HEARD.with_borrow_mut(std::mem::take)
}

#[test]
fn four_observers_install_and_hear_each_report_in_install_order() {
    let slots = empty_slots();

    for observer in [&FIRST, &SECOND, &THIRD, &FOURTH] {
        assert!(observe_in(&slots, observer, false));
    }

    assert_eq!(heard(&slots), ["first", "second", "third", "fourth"]);
}

#[test]
fn a_fifth_observer_is_refused_and_the_four_installed_stay() {
    let slots = empty_slots();
    for observer in [&FIRST, &SECOND, &THIRD, &FOURTH] {
        observe_in(&slots, observer, false);
    }

    assert!(!observe_in(&slots, &FIFTH, false));

    assert_eq!(heard(&slots), ["first", "second", "third", "fourth"]);
}

#[test]
fn a_run_that_answers_a_completion_request_installs_no_observer() {
    let slots = empty_slots();

    assert!(!observe_in(&slots, &FIRST, true));

    assert_eq!(heard(&slots), Vec::<&str>::new());
}

#[test]
fn an_observer_installed_after_a_report_hears_only_the_later_reports() {
    let slots = empty_slots();
    observe_in(&slots, &FIRST, false);
    let before = heard(&slots);

    observe_in(&slots, &SECOND, false);

    assert_eq!(before, ["first"]);
    assert_eq!(heard(&slots), ["first", "second"]);
}

#[test]
fn the_interrupt_is_handled_when_any_installed_observer_handles_it() {
    static HANDLES: Named = Named {
        interrupt: true,
        ..plain("handles")
    };
    let slots = empty_slots();
    observe_in(&slots, &FIRST, false);
    assert!(!handles_interrupt_in(&slots));

    observe_in(&slots, &HANDLES, false);

    assert!(handles_interrupt_in(&slots));
}

#[test]
fn child_variables_come_in_install_order_and_the_first_observer_to_give_a_name_keeps_it() {
    static TRACING: Named = Named {
        env: &[("TRACEPARENT", "00-first"), ("TRACESTATE", "")],
        ..plain("tracing")
    };
    static OTHER: Named = Named {
        env: &[("TRACEPARENT", "00-second"), ("OTHER", "1")],
        ..plain("other")
    };
    let slots = empty_slots();
    observe_in(&slots, &TRACING, false);
    observe_in(&slots, &OTHER, false);

    let env = child_env_in(&slots);

    assert_eq!(
        env,
        [
            ("TRACEPARENT", "00-first".to_owned()),
            ("TRACESTATE", String::new()),
            ("OTHER", "1".to_owned()),
        ]
    );
}

#[cfg(feature = "tracing")]
#[test]
fn the_trace_ids_come_from_the_first_installed_observer_that_gives_them() {
    static FIRST_IDS: Named = Named {
        ids: Some(("trace-a", "run-a")),
        ..plain("first-ids")
    };
    static SECOND_IDS: Named = Named {
        ids: Some(("trace-b", "run-b")),
        ..plain("second-ids")
    };
    let slots = empty_slots();
    observe_in(&slots, &FIRST, false);
    observe_in(&slots, &FIRST_IDS, false);
    observe_in(&slots, &SECOND_IDS, false);

    let ids = super::trace_ids_in(&slots).unwrap();

    assert_eq!((ids.trace(), ids.run()), ("trace-a", "run-a"));
}

const TIMED_OUT: Code = Code::new("shelf", "network", "timed_out");

/// A guide that maps `TIMED_OUT` to 124 and refused command lines to 64
/// (ADR-0037), with the smallest output budget.
const GUIDE: Guide = Guide::new()
    .spelling(DOTS)
    .output_budget(1024, "the tests print little")
    .exits(&[
        Exit::new(124, "The catalogue ran out of time.").failures(&[&TIMED_OUT]),
        Exit::new(64, "The command line was refused.").failures(&[&crate::codes::CLI_USAGE]),
    ]);

/// A tool with `books list`, which takes `--all`, and `books show`, which
/// takes ids.
#[derive(Parser)]
#[command(name = "shelf", version = "1.2.3")]
struct Tool {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    view: crate::ViewFlags,
}

#[derive(Subcommand)]
enum Command {
    /// Work with books.
    Books {
        #[command(subcommand)]
        command: Books,
    },
}

#[derive(Subcommand)]
enum Books {
    /// List the books.
    List {
        /// List every book.
        #[arg(long)]
        all: bool,
    },
    /// Show books by id.
    Show {
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
    },
}

/// Parses `args` into memory with [`GUIDE`], which sets the spelling
/// [`DOTS`], and
/// returns the output flags with the tool's command, or the exit code of a
/// run that parsing ended.
fn parse(args: &[&str]) -> Result<Output, ExitCode> {
    let (_, output) = parse_to::<Tool, _, _, _, _>(
        &mut Sink::new(Vec::new(), Vec::new()),
        Tool::command(),
        &GUIDE,
        args,
    )?;
    Ok(output)
}

#[test]
fn parse_reports_the_root_command_and_the_typed_words_before_it_reports_the_parsed_line() {
    let seen = watch(|| {
        let _ = parse(&["shelf", "books", "list", "--all"]);
    });

    assert_eq!(
        seen,
        [
            "typed shelf: shelf books list --all in a.b.c",
            "parsed shelf books list",
        ]
    );
}

#[test]
fn parse_reports_a_help_page_with_its_command_path_whether_it_is_long_and_its_exit() {
    let seen = watch(|| {
        let _ = parse(&["shelf", "books", "list", "-h"]);
        let _ = parse(&["shelf", "books", "--help"]);
        let _ = parse(&["shelf", "help"]);
    });

    let helped: Vec<&String> = seen
        .iter()
        .filter(|event| event.starts_with("helped"))
        .collect();
    assert_eq!(
        helped,
        [
            "helped shelf: books list short 0",
            "helped shelf: books long 0",
            "helped shelf:  long 0",
        ]
    );
}

#[test]
fn parse_reports_the_version_as_an_exit_with_code_0() {
    let seen = watch(|| {
        let _ = parse(&["shelf", "--version"]);
    });

    assert_eq!(seen[1..], ["exited 0"]);
}

#[test]
fn a_refusal_reports_the_guide_s_exit_code_and_the_visible_flag_that_clap_suggested() {
    let seen = watch(|| {
        let _ = parse(&["shelf", "books", "list", "--al"]);
    });

    assert_eq!(
        seen[1..],
        [
            "refused shelf: books list 64 nearest all",
            "failed shelf.cli.usage"
        ]
    );
}

/// A book.
#[derive(Serialize, JsonSchema)]
struct Book {
    title: String,
}

impl Row for Book {
    const COLUMNS: &'static [&'static str] = &["title"];
}

const BOOKS: Document<Listing<Book>> = Document::new("shelf", "books-list", 1, "The books.");

/// Returns a listing of `count` books out of `total`.
fn books(count: usize, total: usize) -> Listing<Book> {
    let rows = (0..count)
        .map(|n| Book {
            title: format!("Book number {n}"),
        })
        .collect();
    Listing::new(rows, total)
}

/// A failure that runs out of time.
#[derive(Debug)]
struct TimedOut;

impl fmt::Display for TimedOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the catalogue ran out of time")
    }
}

impl std::error::Error for TimedOut {}

impl Failure for TimedOut {
    fn code(&self) -> &'static Code {
        &TIMED_OUT
    }

    fn recovery(&self) -> Recovery {
        Recovery::Retry
    }
}

/// Parses `args` and finishes `result` into memory, and returns the events
/// that `finish` reported and the bytes it wrote to standard output.
fn finished<T: crate::Rows, E: crate::AsFailure>(
    args: &[&str],
    document: Document<T>,
    result: Result<T, E>,
) -> (Vec<String>, usize) {
    let output = parse(args).unwrap();
    let mut out = Vec::new();
    let seen = watch(|| {
        let sink = &mut Sink::new(&mut out, Vec::new());
        let _ = finish_to(sink, output, document, result);
    });
    (seen, out.len())
}

#[test]
fn finish_reports_its_exit_code_its_bytes_and_whether_the_listing_left_rows_out_or_was_empty() {
    let (whole, bytes) = finished(
        &["shelf", "books", "list"],
        BOOKS,
        Ok::<_, Error>(books(2, 2)),
    );
    let (partial, _) = finished(
        &["shelf", "books", "list"],
        BOOKS,
        Ok::<_, Error>(books(2, 5)),
    );
    let (empty, _) = finished(
        &["shelf", "books", "list"],
        BOOKS,
        Ok::<_, Error>(books(0, 0)),
    );

    assert_eq!(
        whole,
        [format!(
            "finished 0 {bytes} bytes truncated false empty false"
        )]
    );
    assert!(
        partial[0].ends_with("truncated true empty false"),
        "{partial:?}"
    );
    assert!(
        empty[0].ends_with("truncated false empty true"),
        "{empty:?}"
    );
}

/// Returns an uncounted listing of `count` books whose source holds no
/// rows after them, as `Page::uncounted` returns with `--all`.
fn every_uncounted_book(count: usize) -> Listing<Book> {
    let rows = (0..count)
        .map(|n| Book {
            title: format!("Book number {n}"),
        })
        .collect();
    Listing::uncounted(rows, false)
}

#[test]
fn finish_reports_an_uncounted_listing_with_no_rows_and_no_more_rows_as_empty() {
    let (seen, _) = finished(
        &["shelf", "books", "list"],
        BOOKS,
        Ok::<_, Error>(every_uncounted_book(0)),
    );

    assert!(seen[0].ends_with("truncated false empty true"), "{seen:?}");
}

#[test]
fn finish_reports_an_uncounted_listing_that_holds_every_row_as_not_truncated() {
    let (seen, _) = finished(
        &["shelf", "books", "list"],
        BOOKS,
        Ok::<_, Error>(every_uncounted_book(3)),
    );

    assert!(seen[0].ends_with("truncated false empty false"), "{seen:?}");
}

#[test]
fn finish_reports_the_failure_s_code_in_the_tool_s_spelling_and_the_guide_s_exit_code() {
    let (seen, _) = finished(&["shelf", "books", "list"], BOOKS, Err(TimedOut));

    assert_eq!(
        seen,
        [
            "failed shelf.network.timed_out",
            "finished 124 0 bytes truncated false empty false"
        ]
    );
}

#[test]
fn a_listing_that_the_budget_cut_reports_the_budget_s_code_and_exits_with_0() {
    let (seen, bytes) = finished(
        &["shelf", "books", "list"],
        BOOKS,
        Ok::<_, Error>(books(2000, 2000)),
    );

    assert_eq!(
        seen,
        [
            "failed shelf.output.over_budget".to_owned(),
            format!("finished 0 {bytes} bytes truncated true empty false"),
        ]
    );
}

const STREAMED: Document<Stream<Book>> = Document::new("shelf", "books-watch", 1, "The books.");

#[test]
fn a_stream_reports_its_exit_its_bytes_and_whether_the_budget_stopped_it() {
    let output = parse(&["shelf", "books", "list"]).unwrap();
    let mut out = Vec::new();

    let seen = watch(|| {
        let sink = &mut Sink::new(&mut out, Vec::new());
        let mut streaming = stream_to(sink, output, STREAMED).unwrap();
        let _ = streaming.item(&Book {
            title: "Dune".to_owned(),
        });
        let _ = streaming.end(Err(TimedOut));
    });

    assert_eq!(
        seen,
        [
            "failed shelf.network.timed_out".to_owned(),
            format!(
                "finished 124 {} bytes truncated false empty false",
                out.len()
            ),
        ]
    );
}

static LOCKED: Code = Code::new("shelf", "catalogue", "locked");

#[test]
fn the_public_reports_reach_the_observer_and_ended_returns_its_exit_code() {
    let mut code = ExitCode::SUCCESS;

    let seen = watch(|| {
        super::aliased();
        super::tolerated(&LOCKED);
        code = super::ended(124, Some(2048));
    });

    assert_eq!(code, ExitCode::from(124));
    assert_eq!(
        seen,
        [
            "aliased",
            "tolerated shelf.catalogue.locked",
            "ended 124 Some(2048)"
        ]
    );
}

#[test]
fn the_hand_over_reports_reach_the_observer_and_no_test_observer_handles_interrupt() {
    let mut handles = true;

    let seen = watch(|| {
        super::handed_over();
        super::interrupted();
        super::taken_back();
        handles = super::handles_interrupt();
    });

    assert!(!handles);
    assert_eq!(seen, ["handed over", "interrupted", "taken back"]);
}

#[test]
fn child_env_returns_the_variables_that_the_observer_gives() {
    let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-2222222222222222-01";
    give_child_env(vec![("TRACEPARENT", traceparent.to_owned())]);

    let env = crate::trace::child_env();

    give_child_env(Vec::new());
    assert_eq!(env, [("TRACEPARENT", traceparent.to_owned())]);
}
