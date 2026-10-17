use super::{Ran, judge, judge_text};

/// Returns a run that exited with `code` and printed `stdout` and `stderr`.
const fn ran<'a>(code: i32, stdout: &'a str, stderr: &'a str) -> Ran<'a> {
    Ran {
        code: Some(code),
        stdout,
        stderr,
    }
}

#[test]
fn a_cut_listing_that_holds_the_budget_keeps_the_contract() {
    let stdout = "{\"schema\":\"shelf.books.list.v1\",\"data\":[],\"cut\":{\"code\":\"sh::output::over_budget\",\"budget\":200}}\n";
    assert_eq!(judge(&ran(0, stdout, ""), 200, 1), None);
}

#[test]
fn an_unwrapped_listing_whose_standard_error_names_the_budget_keeps_the_contract() {
    let stderr = "Showing 1 of 3 books, because output stops at 100 bytes. Ask for less with --limit or --fields.\n";
    assert_eq!(judge(&ran(0, "[{\"id\":1}]\n", stderr), 100, 1), None);
}

#[test]
fn a_result_that_fits_without_marking_the_cut_breaks_the_contract() {
    let problem = judge(&ran(0, "{\"data\":[]}\n", ""), 100, 1).expect("a problem");
    assert!(
        problem.contains("without marking it as cut short"),
        "{problem}"
    );
}

#[test]
fn a_result_past_the_budget_that_exits_0_breaks_the_contract_unless_it_is_one_row() {
    let rows = "{\"data\":[{\"id\":1},{\"id\":2}]}\n";
    let problem = judge(&ran(0, rows, ""), 10, 1).expect("a problem");
    assert!(problem.contains("printed 29 bytes"), "{problem}");
    assert_eq!(
        judge(
            &ran(0, "{\"data\":{\"id\":1,\"name\":\"long\"}}\n", ""),
            10,
            1
        ),
        None
    );
}

#[test]
fn a_batch_of_one_answer_past_the_budget_that_exits_0_keeps_the_contract() {
    let one = "{\"data\":[{\"id\":1,\"name\":\"long\"}],\"failures\":[]}\n";

    assert_eq!(judge(&ran(0, one, ""), 10, 1), None);
}

#[test]
fn a_failure_document_with_the_budget_keeps_the_contract() {
    let stdout = "{\"schema\":\"crumbkit.failure.v1\",\"code\":\"sh::output::over_budget\",\"fields\":{\"budget\":100}}\n";
    assert_eq!(judge(&ran(1, stdout, ""), 100, 1), None);
}

#[test]
fn a_batch_whose_last_failure_holds_the_budget_keeps_the_contract() {
    let stdout = "{\"data\":[{\"id\":1}],\"failures\":[{\"code\":\"x\",\"fields\":{}},{\"code\":\"sh::output::over_budget\",\"fields\":{\"left_out\":2,\"budget\":100}}]}\n";
    assert_eq!(judge(&ran(1, stdout, ""), 100, 1), None);
}

#[test]
fn an_exit_1_without_the_over_budget_failure_breaks_the_contract() {
    let stdout = "{\"code\":\"shelf::books::not_found\",\"fields\":{}}\n";
    let problem = judge(&ran(1, stdout, ""), 100, 1).expect("a problem");
    assert!(
        problem.contains("without the over-budget failure"),
        "{problem}"
    );
}

#[test]
fn a_run_that_ends_with_another_exit_code_breaks_the_contract() {
    let problem = judge(&ran(2, "", ""), 100, 1).expect("a problem");
    assert!(problem.contains("exit code 2"), "{problem}");
    let signalled = Ran {
        code: None,
        stdout: "",
        stderr: "",
    };
    let problem = judge(&signalled, 100, 1).expect("a problem");
    assert!(problem.contains("a signal"), "{problem}");
}

#[test]
fn an_indented_failure_document_with_the_budget_keeps_the_contract() {
    let stdout = "{\n  \"schema\": \"sh/failure/v1\",\n  \"code\": \"sh::output::over_budget\",\n  \"fields\": {\n    \"budget\": 100\n  }\n}\n";
    assert_eq!(judge(&ran(1, stdout, ""), 100, 1), None);
}

#[test]
fn an_indented_cut_listing_that_holds_the_budget_keeps_the_contract() {
    let stdout = "{\n  \"data\": [],\n  \"cut\": {\n    \"budget\": 200\n  }\n}\n";
    assert_eq!(judge(&ran(0, stdout, ""), 200, 1), None);
}

/// The words of an example that succeeds only in a directory that holds the
/// file `marker` and with `input` on standard input, and that exits with
/// code 4 when the check lowers its budget.
#[cfg(unix)]
const PICKY: &[&str] = &[
    r#"test -f marker && test "$(cat)" = input || exit 2; test -n "$SH_OUTPUT_BUDGET" && exit 4; echo printed"#,
    "--json",
];

#[cfg(unix)]
#[test]
fn the_output_budget_check_uses_current_dir_and_stdin_and_skips_an_opted_out_example() {
    use clap::Command;
    use crumbkit::help::{Example, Guide, Leaf};

    use crate::allow::Allow;
    use crate::examples::{Examples, example_golden_name};
    use crate::guide::command_line;
    use crate::temp::TempDir;

    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(PICKY, "Print")])]);
    const ALLOW: &[Allow] = &[Allow::example("-c", PICKY, "The output names the day.")];
    let sh = Command::new("sh").subcommand(Command::new("-c"));
    let goldens = TempDir::new("output-budget-inputs").unwrap();
    let name = example_golden_name(&command_line("sh -c", PICKY));
    goldens.write(&name.replace(".txt", ".stdin"), "input");
    let work = TempDir::new("output-budget-inputs-work").unwrap();
    work.write("marker", "");
    let (dir, path) = (
        goldens.path().display().to_string(),
        work.path().display().to_string(),
    );
    let examples = Examples::new("/bin/sh", &dir).current_dir(&path);

    let problems = super::check(&sh, &GUIDE, &examples, super::TIMEOUT);
    let skipped = super::check(&sh, &GUIDE, &examples.allow(ALLOW), super::TIMEOUT);

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains("ended with exit code 4"),
        "{problems:?}"
    );
    assert_eq!(skipped, Vec::<String>::new());
}

/// A `--json` example that prints a line, and that never ends when the
/// output budget is set.
#[cfg(unix)]
const STALLS: &[&str] = &[
    r#"test -n "$SH_OUTPUT_BUDGET" && exec sleep 5; echo printed"#,
    "--json",
];

#[cfg(unix)]
#[test]
fn a_run_with_the_budget_that_does_not_end_within_the_timeout_is_stopped_and_breaks_the_contract() {
    use std::time::Duration;

    use clap::Command;
    use crumbkit::help::{Example, Guide, Leaf};

    use crate::examples::Examples;
    use crate::temp::TempDir;

    const GUIDE: Guide =
        Guide::new().leaves(&[Leaf::new(&["-c"]).examples(&[Example::new(STALLS, "Print")])]);
    let sh = Command::new("sh").subcommand(Command::new("-c"));
    let goldens = TempDir::new("output-budget-stalls").unwrap();
    let dir = goldens.path().display().to_string();

    let problems = super::check(
        &sh,
        &GUIDE,
        &Examples::new("/bin/sh", &dir),
        Duration::from_millis(200),
    );

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].ends_with(
            "with an output budget of 4 bytes, the run did not end within 0.2 seconds, so the check stopped it"
        ),
        "{problems:?}"
    );
}

#[test]
fn a_guide_that_gives_the_over_budget_failure_another_exit_code_makes_that_code_the_failed_one() {
    let stdout = "{\"code\":\"sh::output::over_budget\",\"fields\":{\"budget\":100}}\n";
    assert_eq!(judge(&ran(9, stdout, ""), 100, 9), None);
    let problem = judge(&ran(1, stdout, ""), 100, 9).expect("a problem");
    assert!(problem.contains("exit code 1"), "{problem}");
}

/// A `--json` example that prints a line, and that fails past its budget
/// with a document that names the budget and exits with code 9.
#[cfg(unix)]
const FAILS_WITH_9: &[&str] = &[
    r#"test -n "$SH_OUTPUT_BUDGET" && echo "{\"fields\":{\"budget\":$SH_OUTPUT_BUDGET}}" && exit 9; echo printed"#,
    "--json",
];

#[cfg(unix)]
#[test]
fn a_leaf_whose_rename_of_the_over_budget_failure_an_exit_lists_makes_that_code_the_failed_one() {
    use clap::Command;
    use crumbkit::Code;
    use crumbkit::help::{Example, Exit, Guide, Leaf, Rename};

    use crate::examples::Examples;
    use crate::temp::TempDir;

    const TOO_LONG: Code = Code::new("sh", "output", "too_long");
    const GUIDE: Guide = Guide::new()
        .exits(&[Exit::new(9, "The output was too long.").failures(&[&TOO_LONG])])
        .leaves(&[Leaf::new(&["-c"])
            .examples(&[Example::new(FAILS_WITH_9, "Print")])
            .codes(&[Rename::new(&crumbkit::codes::OUTPUT_OVER_BUDGET, &TOO_LONG)])]);
    let sh = Command::new("sh").subcommand(Command::new("-c"));
    let goldens = TempDir::new("output-budget-leaf-exit").unwrap();
    let dir = goldens.path().display().to_string();

    let problems = super::check(&sh, &GUIDE, &Examples::new("/bin/sh", &dir), super::TIMEOUT);

    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn a_headed_text_run_that_fails_and_names_the_budget_on_standard_error_keeps_the_contract() {
    let stderr = "error: 2 more answers did not fit in the output budget of 100 bytes\n";
    let stdout = "==> a <==\none\n";

    assert_eq!(
        judge_text(&ran(1, stdout, stderr), stdout.len(), 100, 1, false),
        None
    );
}

#[test]
fn a_headed_text_run_past_the_budget_keeps_the_contract_only_when_its_document_always_prints_its_first_answer()
 {
    let stdout = "x".repeat(150);
    let run = ran(0, &stdout, "");

    let problem = judge_text(&run, 150, 100, 1, false).expect("a problem");
    assert!(problem.contains("printed 150 bytes"), "{problem}");
    assert_eq!(judge_text(&run, 150, 100, 1, true), None);
}

#[test]
fn a_headed_text_run_that_exits_0_within_the_budget_breaks_the_contract() {
    let problem = judge_text(&ran(0, "one\n", ""), 4, 100, 1, false).expect("a problem");

    assert!(
        problem.contains("without the over-budget failure"),
        "{problem}"
    );
}

/// A text example that prints a line, and that exits with code 3 when the
/// output budget is set.
#[cfg(unix)]
const TEXT_FAILS_WITH_3: &[&str] = &[r#"test -n "$SH_OUTPUT_BUDGET" && exit 3; echo printed"#];

/// A row that holds a file's bytes, which a headed document prints under a
/// header.
#[cfg(unix)]
#[derive(serde::Serialize, schemars::JsonSchema)]
struct File {
    /// The file's name.
    name: String,
    /// The file's bytes.
    #[serde(flatten)]
    content: crumbkit::Bytes,
}

#[cfg(unix)]
impl crumbkit::Row for File {
    const COLUMNS: &'static [&'static str] = &["name"];
}

#[cfg(unix)]
impl crumbkit::Headed for File {
    fn bytes(&self) -> Option<&crumbkit::Bytes> {
        Some(&self.content)
    }
}

#[cfg(unix)]
#[test]
fn the_output_budget_check_runs_the_text_examples_of_a_headed_leaf() {
    use clap::Command;
    use crumbkit::help::{Example, Guide, Leaf};
    use crumbkit::{Batch, Document, Error};

    use crate::examples::Examples;
    use crate::temp::TempDir;

    const FILES: Document<Batch<File, Error>> =
        Document::new("sh", "files", 1, "The files.").headed();
    const GUIDE: Guide = Guide::new().leaves(&[Leaf::new(&["-c"])
        .examples(&[Example::new(TEXT_FAILS_WITH_3, "Print")])
        .document(FILES)]);
    let sh = Command::new("sh").subcommand(Command::new("-c"));
    let goldens = TempDir::new("output-budget-headed").unwrap();
    let dir = goldens.path().display().to_string();

    let problems = super::check(&sh, &GUIDE, &Examples::new("/bin/sh", &dir), super::TIMEOUT);

    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains("ended with exit code 3"),
        "{problems:?}"
    );
}
