//! Tests for reading exported usage and ranking its commands.

use super::{Ranked, Run, parse, rank, read};

/// Returns a run of `command` in `session` at second `second`, with `exit`
/// and `flags`.
fn run(session: &str, second: u32, command: &str, exit: u8, flags: &[&str]) -> Run {
    Run {
        id: None,
        tool: "shelf".to_owned(),
        session: session.to_owned(),
        at: format!("2026-11-01T09:00:{second:02}.000Z"),
        command: Some(command.to_owned()),
        flags: flags.iter().map(|&flag| flag.to_owned()).collect(),
        failed: exit != 0,
        help: false,
    }
}

/// Returns the commands of `ranked` with their runs and wasted runs.
fn figures(ranked: &[Ranked]) -> Vec<(&str, u64, u64)> {
    ranked
        .iter()
        .map(|ranked| (ranked.command.as_str(), ranked.runs, ranked.wasted))
        .collect()
}

#[test]
fn commands_rank_by_runs_plus_the_runs_that_failed_or_were_retried() {
    let runs = [
        // Three clean runs of `books list`.
        run("a", 0, "books list", 0, &[]),
        run("a", 1, "books list", 0, &[]),
        run("b", 0, "books list", 0, &[]),
        // `books show` fails and is run again: two runs, both wasted.
        run("a", 2, "books show", 1, &[]),
        run("a", 3, "books show", 0, &[]),
    ];

    let ranked = rank(&runs);

    // books show scores 2 × (1 + 2/2) = 4, and books list 3 × (1 + 0) = 3.
    assert_eq!(
        figures(&ranked),
        [("books show", 2, 2), ("books list", 3, 0)]
    );
}

#[test]
fn a_run_is_a_retry_only_right_after_a_failed_run_of_the_same_command_in_its_session() {
    let runs = [
        run("a", 0, "books show", 1, &[]),
        run("a", 1, "books list", 0, &[]),
        run("a", 2, "books show", 0, &[]),
        // Another session's run after the failure is no retry.
        run("b", 0, "books show", 0, &[]),
    ];

    let ranked = rank(&runs);

    assert_eq!(
        figures(&ranked),
        [("books show", 3, 1), ("books list", 1, 0)]
    );
}

#[test]
fn runs_order_by_time_within_a_session_whatever_order_the_files_hold() {
    let runs = [
        run("a", 5, "books show", 0, &[]),
        run("a", 1, "books show", 1, &[]),
    ];

    let ranked = rank(&runs);

    assert_eq!(figures(&ranked), [("books show", 2, 2)]);
}

#[test]
fn help_pages_and_runs_of_the_root_are_not_ranked_and_do_not_break_a_retry() {
    let mut help = run("a", 1, "books show", 0, &[]);
    help.help = true;
    let mut root = run("a", 2, "books", 0, &[]);
    root.command = None;
    let runs = [
        run("a", 0, "books show", 1, &[]),
        help,
        root,
        run("a", 3, "books show", 0, &[]),
    ];

    let ranked = rank(&runs);

    assert_eq!(figures(&ranked), [("books show", 2, 2)]);
}

#[test]
fn equal_scores_rank_by_command_and_flags_rank_by_use_then_name() {
    let runs = [
        run("a", 0, "books show", 0, &["--from"]),
        run("a", 1, "books list", 0, &["--json", "--limit"]),
        run("a", 2, "books list", 0, &["--limit"]),
        run("a", 3, "books show", 0, &["--json"]),
    ];

    let ranked = rank(&runs);

    assert_eq!(
        figures(&ranked),
        [("books list", 2, 0), ("books show", 2, 0)]
    );
    assert_eq!(ranked[0].flags, ["--limit", "--json"]);
    assert_eq!(ranked[1].flags, ["--from", "--json"]);
}

#[test]
fn an_exported_line_gives_its_stored_record_as_a_run() {
    let line = r#"{"schema":"shelf/self-runs-export/v1","data":{"gap":false,"record":{"schema":"shelf/usage/v1","tool":"shelf","at":"2026-11-01T09:00:00.000Z","session":"6d0d3b8ba9f7d735","run":"04bdd5e48d990113","command":"books list","flags":["--all"],"exit":2}}}"#;

    let run = parse(line).unwrap();

    assert_eq!(run.id.as_deref(), Some("04bdd5e48d990113"));
    assert_eq!(run.tool, "shelf");
    assert_eq!(run.command.as_deref(), Some("books list"));
    assert_eq!(run.flags, ["--all"]);
    assert!(run.failed);
    assert!(!run.help);
}

#[test]
fn a_line_that_is_not_an_exported_record_gives_none() {
    let stored = r#"{"schema":"shelf/usage/v1","tool":"shelf","at":"2026-11-01T09:00:00.000Z","session":"6d0d3b8ba9f7d735"}"#;

    assert!(parse(stored).is_none());
    assert!(parse("not json").is_none());
}

#[test]
fn a_usage_directory_that_does_not_exist_fails_with_its_path() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("usage");

    let (path, error) = read(&missing).unwrap_err();

    assert_eq!(path, missing);
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn a_usage_directory_reads_jsonl_files_in_subdirectories_each_run_once() {
    let dir = tempfile::tempdir().unwrap();
    let line = r#"{"schema":"shelf/self-runs-export/v1","data":{"gap":false,"record":{"tool":"shelf","at":"2026-11-01T09:00:00.000Z","session":"a","run":"04bdd5e48d990113","command":"books list","exit":0}}}"#;
    let host = dir.path().join("host-a");
    #[expect(
        clippy::disallowed_methods,
        reason = "the test writes the usage files that it reads"
    )]
    {
        std::fs::create_dir_all(&host).unwrap();
        std::fs::write(host.join("1.jsonl"), format!("{line}\nnot json\n")).unwrap();
        std::fs::write(dir.path().join("2.jsonl"), format!("{line}\n")).unwrap();
        std::fs::write(dir.path().join("notes.txt"), "not read").unwrap();
    }

    let usage = read(dir.path()).unwrap();

    assert_eq!(usage.runs.len(), 1);
    assert_eq!(usage.unreadable, 1);
}
