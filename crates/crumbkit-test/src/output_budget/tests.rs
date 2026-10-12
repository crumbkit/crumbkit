use super::{Ran, judge};

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
    let stdout = "{\"schema\":\"shelf.books.list.v1\",\"data\":[],\"cut\":{\"code\":\"crumbkit::output::over_budget\",\"budget\":200}}\n";
    assert_eq!(judge(&ran(0, stdout, ""), 200), None);
}

#[test]
fn an_unwrapped_listing_whose_standard_error_names_the_budget_keeps_the_contract() {
    let stderr = "Showing 1 of 3 books, because output stops at 100 bytes. Ask for less with --limit or --fields.\n";
    assert_eq!(judge(&ran(0, "[{\"id\":1}]\n", stderr), 100), None);
}

#[test]
fn a_result_that_fits_without_marking_the_cut_breaks_the_contract() {
    let problem = judge(&ran(0, "{\"data\":[]}\n", ""), 100).expect("a problem");
    assert!(
        problem.contains("without marking it as cut short"),
        "{problem}"
    );
}

#[test]
fn a_result_past_the_budget_that_exits_0_breaks_the_contract_unless_it_is_one_row() {
    let rows = "{\"data\":[{\"id\":1},{\"id\":2}]}\n";
    let problem = judge(&ran(0, rows, ""), 10).expect("a problem");
    assert!(problem.contains("printed 29 bytes"), "{problem}");
    assert_eq!(
        judge(&ran(0, "{\"data\":{\"id\":1,\"name\":\"long\"}}\n", ""), 10),
        None
    );
}

#[test]
fn a_failure_document_with_the_budget_keeps_the_contract() {
    let stdout = "{\"schema\":\"crumbkit.failure.v1\",\"code\":\"crumbkit::output::over_budget\",\"fields\":{\"budget\":100}}\n";
    assert_eq!(judge(&ran(1, stdout, ""), 100), None);
}

#[test]
fn a_batch_whose_last_failure_holds_the_budget_keeps_the_contract() {
    let stdout = "{\"data\":[{\"id\":1}],\"failures\":[{\"code\":\"x\",\"fields\":{}},{\"code\":\"crumbkit::output::over_budget\",\"fields\":{\"left_out\":2,\"budget\":100}}]}\n";
    assert_eq!(judge(&ran(1, stdout, ""), 100), None);
}

#[test]
fn an_exit_1_without_the_over_budget_failure_breaks_the_contract() {
    let stdout = "{\"code\":\"shelf::books::not_found\",\"fields\":{}}\n";
    let problem = judge(&ran(1, stdout, ""), 100).expect("a problem");
    assert!(
        problem.contains("without the over-budget failure"),
        "{problem}"
    );
}

#[test]
fn a_run_that_ends_with_another_exit_code_breaks_the_contract() {
    let problem = judge(&ran(2, "", ""), 100).expect("a problem");
    assert!(problem.contains("exit code 2"), "{problem}");
    let signalled = Ran {
        code: None,
        stdout: "",
        stderr: "",
    };
    let problem = judge(&signalled, 100).expect("a problem");
    assert!(problem.contains("a signal"), "{problem}");
}

#[test]
fn an_indented_failure_document_with_the_budget_keeps_the_contract() {
    let stdout = "{\n  \"schema\": \"crumbkit/failure/v1\",\n  \"code\": \"crumbkit::output::over_budget\",\n  \"fields\": {\n    \"budget\": 100\n  }\n}\n";
    assert_eq!(judge(&ran(1, stdout, ""), 100), None);
}

#[test]
fn an_indented_cut_listing_that_holds_the_budget_keeps_the_contract() {
    let stdout = "{\n  \"data\": [],\n  \"cut\": {\n    \"budget\": 200\n  }\n}\n";
    assert_eq!(judge(&ran(0, stdout, ""), 200), None);
}
