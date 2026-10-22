use std::fs;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use clap::Command;
use serde_json::{Value, json};

use super::{
    Recorder, State, child_env_in, install, save, take, update, wait_for_line, write, written,
};
use crate::caller::Caller;
use crate::context::Context;
use crate::file::LIMITS;
use crate::record::Help;
use crate::testing::TempDir;
use crate::{Extension, Facts, Key};
use crumbkit::{Code, Spelling, ToolName};

/// Joins the segments of a code with `.`, so a test shows that the code
/// prints in the spelling that the tool's guide sets, where the colon
/// spelling would join them with `::`.
const DOTS: Spelling =
    Spelling::custom(|code, f| write!(f, "{}.{}.{}", code.component(), code.area(), code.reason()));

mod processes;

/// Returns a tool's command with a version.
fn tool() -> Command {
    Command::new("shelf").version("1.2.3")
}

/// The W3C example TRACEPARENT, which names a valid caller's span.
const CALLER: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

/// Returns the recorder of a run that started now, under a caller that set
/// `traceparent`, with the run id `2222222222222222`.
fn started(traceparent: Option<&str>) -> Recorder {
    let context = Context::new(traceparent, None, [0x11; 16], [0x22; 8]).unwrap();
    Recorder::new(Instant::now(), context)
}

/// Returns the recorder of a run of [`tool`] that reached `parse` with no
/// words after the tool's name.
fn parsed() -> Recorder {
    let mut recorder = started(None);
    // crumbkit passes the spelling that names the tool (ADR-0079).
    let spelling = DOTS.for_tool(ToolName::new("shelf"));
    recorder.typed(&tool(), &["shelf".into()], spelling);
    recorder
}

/// Returns a person in a shell whose process id is 4242.
fn person() -> Caller {
    Caller {
        name: "person",
        session: "4242".to_owned(),
    }
}

/// Saves the line of `recorder` under `state`, and returns the lines of the
/// tool's log.
fn saved(state: &TempDir, recorder: Recorder, interrupted: bool) -> Vec<Value> {
    let now = SystemTime::UNIX_EPOCH + Duration::from_millis(1_791_374_467_120);
    save(state.path(), recorder, now, interrupted, &person(), &LIMITS).unwrap();
    let log = fs::read_to_string(state.path().join("shelf").join("usage.jsonl")).unwrap();
    log.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn a_second_start_in_one_process_installs_no_second_recorder() {
    let slot = Mutex::new(State::Idle);

    assert!(install(&slot, started(None)));
    assert!(!install(&slot, started(None)));
    take(&slot).unwrap();
    assert!(!install(&slot, started(None)));
}

#[test]
fn without_a_recorder_a_report_does_nothing_and_there_is_no_line_to_write() {
    let slot = Mutex::new(State::Idle);

    update(&slot, |_| {
        panic!("a report reached a recorder that was never installed")
    });

    assert!(take(&slot).is_none());
}

#[test]
fn a_run_gives_up_its_recorder_once_so_it_writes_one_line() {
    let slot = Mutex::new(State::Idle);
    install(&slot, started(None));

    assert!(take(&slot).is_some());
    assert!(take(&slot).is_none());
}

#[test]
fn a_taken_recorder_counts_as_writing_until_its_line_is_written() {
    let slot = Mutex::new(State::Idle);
    install(&slot, started(None));

    take(&slot).unwrap();
    assert!(matches!(*slot.lock().unwrap(), State::Writing));
    written(&slot);

    assert!(matches!(*slot.lock().unwrap(), State::Written));
}

#[test]
fn a_thread_that_ends_the_run_while_another_writes_the_line_waits_until_it_is_written() {
    let slot = Mutex::new(State::Idle);
    install(&slot, started(None));
    take(&slot).unwrap();
    let waited = Instant::now();

    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(30));
            written(&slot);
        });
        write(&slot, false);
    });

    assert!(waited.elapsed() >= Duration::from_millis(30));
    assert!(matches!(*slot.lock().unwrap(), State::Written));
}

#[test]
fn the_signal_thread_stops_waiting_for_a_line_in_progress_after_its_bound() {
    let slot = Mutex::new(State::Idle);
    install(&slot, started(None));
    take(&slot).unwrap();
    let waited = Instant::now();

    let finished = wait_for_line(&slot, Duration::from_millis(30));

    assert!(!finished);
    assert!(waited.elapsed() >= Duration::from_millis(30));
}

#[test]
fn a_saved_line_holds_the_tool_its_version_the_end_time_the_caller_and_the_hashes() {
    let state = TempDir::new("saved");
    let mut recorder = parsed();
    recorder.words = vec!["books".into(), "show".into(), "s3cr3t".into()];

    let lines = saved(&state, recorder, false);

    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert_eq!(line["schema"], "shelf/usage/v1");
    assert_eq!(line["tool"], "shelf");
    assert_eq!(line["version"], "1.2.3");
    assert_eq!(line["at"], "2026-10-07T12:01:07.120Z");
    assert!(line["ms"].is_u64());
    assert_eq!(line["caller"], "person");
    assert_eq!(line["session"].as_str().unwrap().len(), 16);
    assert_eq!(line["args"].as_str().unwrap().len(), 16);
    assert_eq!(line.get("interrupted"), None);
    assert!(
        !fs::read_to_string(state.path().join("shelf").join("usage.jsonl"))
            .unwrap()
            .contains("s3cr3t")
    );
}

#[test]
fn a_saved_line_holds_the_caller_s_trace_the_run_s_id_and_the_caller_s_span_as_parent() {
    let state = TempDir::new("traced");
    let mut recorder = started(Some(CALLER));
    recorder.typed(&tool(), &["shelf".into()], DOTS);

    let lines = saved(&state, recorder, false);

    assert_eq!(lines[0]["trace"], "4bf92f3577b34da6a3ce929d0e0e4736");
    assert_eq!(lines[0]["run"], "2222222222222222");
    assert_eq!(lines[0]["parent"], "00f067aa0ba902b7");
}

#[test]
fn a_child_gets_trace_variables_only_from_a_run_that_reached_parse() {
    let slot = Mutex::new(State::Idle);
    assert_eq!(child_env_in(&slot), Vec::new());

    install(&slot, started(Some(CALLER)));
    assert_eq!(child_env_in(&slot), Vec::new());

    update(&slot, |recorder| {
        recorder.typed(&tool(), &["shelf".into()], DOTS);
    });
    assert_eq!(
        child_env_in(&slot),
        [(
            "TRACEPARENT",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-2222222222222222-01".to_owned()
        )]
    );

    take(&slot).unwrap();
    assert_eq!(child_env_in(&slot), Vec::new());
}

#[test]
fn a_run_that_never_reached_parse_writes_no_line() {
    let state = TempDir::new("unnamed");
    let now = SystemTime::UNIX_EPOCH + Duration::from_millis(1_791_374_467_120);

    let written = save(state.path(), started(None), now, false, &person(), &LIMITS);

    assert!(written.is_none());
    assert!(!state.path().exists());
}

#[test]
fn parse_gives_the_recorder_the_tool_s_name_its_version_and_the_words_after_the_name() {
    let mut recorder = started(None);

    recorder.typed(
        &tool(),
        &["shelf".into(), "books".into(), "list".into()],
        DOTS,
    );

    assert_eq!(recorder.tool.unwrap().as_str(), "shelf");
    assert_eq!(recorder.version.unwrap().as_str(), "1.2.3");
    assert_eq!(recorder.words, ["books", "list"]);
}

#[test]
fn two_runs_with_the_same_words_and_session_share_their_hashes() {
    let state = TempDir::new("hashes");
    let words = || vec!["books".into(), "list".into()];
    let mut first = parsed();
    first.words = words();
    saved(&state, first, false);
    let mut second = parsed();
    second.words = words();

    let lines = saved(&state, second, true);

    assert_eq!(lines[0]["args"], lines[1]["args"]);
    assert_eq!(lines[0]["session"], lines[1]["session"]);
    assert_eq!(lines[1]["interrupted"], true);
}

#[test]
fn help_records_the_commands_of_its_path_that_the_tool_defines_and_short_or_long() {
    let mut recorder = parsed();
    let tool = tool().subcommand(Command::new("books").subcommand(Command::new("show")));

    recorder.helped(
        &tool,
        &["books".to_owned(), "s3cr3t".to_owned(), "show".to_owned()],
        true,
        0,
    );

    assert_eq!(recorder.run.command.unwrap().as_str(), "books show");
    assert!(matches!(recorder.run.help, Some(Help::Long)));
    assert_eq!(recorder.run.exit, Some(0));
}

#[test]
fn the_first_failure_code_of_a_run_stays_in_the_tool_s_spelling() {
    let mut recorder = parsed();

    recorder.failed(DOTS, &Code::new("shelf", "books", "not_found"));
    recorder.failed(DOTS, &Code::new("shelf", "books", "gone"));

    assert_eq!(recorder.run.code.unwrap().as_str(), "shelf.books.not_found");
}

/// Ten distinct codes, which a run can report as tolerated.
static CODES: [Code; 10] = [
    Code::new("shelf", "books", "locked"),
    Code::new("shelf", "network", "timed_out"),
    Code::new("shelf", "cache", "stale"),
    Code::new("shelf", "cache", "missing"),
    Code::new("shelf", "books", "gone"),
    Code::new("shelf", "books", "moved"),
    Code::new("shelf", "network", "refused"),
    Code::new("shelf", "network", "reset"),
    Code::new("shelf", "config", "unread"),
    Code::new("shelf", "config", "old"),
];

#[test]
fn a_line_holds_each_tolerated_code_once_in_report_order_in_the_tool_s_spelling() {
    let state = TempDir::new("tolerated-1");
    let mut recorder = parsed();

    recorder.tolerated(&CODES[1]);
    recorder.tolerated(&CODES[0]);
    recorder.tolerated(&CODES[1]);

    assert_eq!(
        saved(&state, recorder, false)[0]["tolerated"],
        json!(["shelf.network.timed_out", "shelf.books.locked"])
    );
}

#[test]
fn a_line_holds_the_first_eight_distinct_tolerated_codes() {
    let state = TempDir::new("tolerated-2");
    let mut recorder = parsed();

    for code in &CODES {
        recorder.tolerated(code);
    }

    let line = &saved(&state, recorder, false)[0];
    assert_eq!(line["tolerated"].as_array().unwrap().len(), 8);
    assert_eq!(line["tolerated"][7], "shelf.network.reset");
}

#[test]
fn a_line_without_tolerated_codes_leaves_the_field_out() {
    let state = TempDir::new("tolerated-3");

    let line = &saved(&state, parsed(), false)[0];

    assert!(line.get("tolerated").is_none(), "{line}");
}

#[test]
fn a_saved_line_holds_the_facts_that_the_tool_s_extension_wrote() {
    const REQUESTS: Key = Key::new("requests");
    #[derive(Default)]
    struct Requests(u64);
    impl Extension for Requests {
        fn write(&self, facts: &mut Facts) {
            facts
                .count(REQUESTS, self.0)
                .waited(Duration::from_millis(5 * self.0));
        }
    }
    let state = TempDir::new("facts");
    let mut recorder = parsed();

    recorder
        .extensions
        .change(|requests: &mut Requests| requests.0 += 2);

    let lines = saved(&state, recorder, false);
    assert_eq!(
        lines[0]["facts"],
        json!({"counts": {"requests": 2}, "waited_ms": 10})
    );
}
