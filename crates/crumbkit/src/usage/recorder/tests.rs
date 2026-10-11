use std::fs;
use std::os::unix::process::ExitStatusExt as _;
use std::process::ExitCode;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use clap::Command;
use serde_json::{Value, json};

use super::{Recorder, State, child_env_in, install, save, start, take, typed, update};
use crate::usage::caller::Caller;
use crate::usage::context::Context;
use crate::usage::file::LIMITS;
use crate::usage::record::Help;
use crate::usage::testing::TempDir;
use crate::{Code, Document, Spelling};

/// The variable that tells the signal test's child process to run.
const CHILD: &str = "CRUMBKIT_USAGE_SIGNAL_CHILD";

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
    recorder.typed(&tool(), &["shelf".into()]);
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
fn a_saved_line_holds_the_tool_its_version_the_end_time_the_caller_and_the_hashes() {
    let state = TempDir::new("saved");
    let mut recorder = parsed();
    recorder.words = vec!["books".into(), "show".into(), "s3cr3t".into()];

    let lines = saved(&state, recorder, false);

    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert_eq!(line["schema"], "crumbkit/usage/v1");
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
    recorder.typed(&tool(), &["shelf".into()]);

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

    update(&slot, |recorder| recorder.typed(&tool(), &["shelf".into()]));
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

    recorder.typed(&tool(), &["shelf".into(), "books".into(), "list".into()]);

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
fn help_records_the_command_it_named_and_short_or_long() {
    let mut recorder = parsed();

    recorder.helped(
        &["books".to_owned(), "show".to_owned()],
        true,
        ExitCode::SUCCESS,
    );

    assert_eq!(recorder.run.command.unwrap().as_str(), "books show");
    assert!(matches!(recorder.run.help, Some(Help::Long)));
    assert_eq!(recorder.run.exit, Some(0));
}

#[test]
fn the_first_failure_code_of_a_run_stays_in_the_tool_s_spelling() {
    let mut recorder = parsed();

    recorder.failed(Spelling::DOTS, &Code::new("shelf", "books", "not_found"));
    recorder.failed(Spelling::DOTS, &Code::new("shelf", "books", "gone"));

    assert_eq!(recorder.run.code.unwrap().as_str(), "shelf.books.not_found");
}

#[test]
fn the_tool_s_section_holds_its_document_id_and_its_value() {
    const EXT: Document<Value> = Document::new("shelf", "usage-ext", 1);
    let mut recorder = parsed();

    recorder.section(EXT, &json!({"requests": 2}));

    let section = serde_json::to_value(recorder.run.ext.unwrap()).unwrap();
    assert_eq!(
        section,
        json!({"schema": "shelf/usage-ext/v1", "data": {"requests": 2}})
    );
}

#[test]
fn a_signal_writes_an_interrupted_line_and_then_ends_the_process_as_the_signal_does() {
    let state = TempDir::new("signal");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test runs this test binary again as the child that receives the signal"
    )]
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "usage::recorder::tests::the_signal_test_s_child_starts_the_log_and_raises_sigterm",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("XDG_STATE_HOME", state.path())
        .status()
        .unwrap();

    assert_eq!(status.signal(), Some(signal_hook::consts::SIGTERM));
    let log = fs::read_to_string(state.path().join("signalled").join("usage.jsonl")).unwrap();
    let lines: Vec<Value> = log
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["tool"], "signalled");
    assert_eq!(lines[0]["interrupted"], true);
}

/// Runs only as the child of the signal test, which sets [`CHILD`]. It
/// starts the log, gives it the tool's name as `parse` does, and raises
/// SIGTERM, which the log's thread answers.
#[test]
fn the_signal_test_s_child_starts_the_log_and_raises_sigterm() {
    #[expect(
        clippy::disallowed_methods,
        reason = "the signal test tells its child process to run through the environment"
    )]
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let _log = start();
    typed(&Command::new("signalled"), &["signalled".into()]);
    signal_hook::low_level::raise(signal_hook::consts::SIGTERM).unwrap();
    // The signal ends the process, so the test never gets past the wait.
    std::thread::sleep(Duration::from_secs(10));
    panic!("SIGTERM did not end the process");
}
