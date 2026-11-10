use clap::Command;

use super::shell::{Env, Shell};
use super::{COMPLETION_CHECK, COMPLETION_INSTALL, COMPLETION_PRINT, print_in};
use crate::schemas::document_schema;

fn shelf() -> Command {
    Command::new("shelf").subcommand(Command::new("books"))
}

#[test]
fn print_prints_the_registration_of_the_shell_that_the_shell_variable_names() {
    let env = Env {
        shell: Some("/usr/bin/fish".into()),
        ..Env::default()
    };

    let prose = print_in(&shelf(), None, &env, "COMPLETE").unwrap();

    assert_eq!(prose.value().shell, Shell::Fish);
    assert!(
        prose.value().text.starts_with("complete --keep-order"),
        "{}",
        prose.value().text
    );
}

#[test]
fn print_serves_elvish_and_powershell_too() {
    for shell in [Shell::Elvish, Shell::Powershell] {
        let prose = print_in(&shelf(), Some(shell), &Env::default(), "COMPLETE").unwrap();

        assert!(prose.value().text.contains("COMPLETE"));
    }
}

#[test]
fn print_refuses_an_unset_shell() {
    let refused = print_in(&shelf(), None, &Env::default(), "COMPLETE").unwrap_err();

    assert_eq!(refused.failure().code(), &crate::codes::SHELL_UNSET);
}

#[test]
fn the_checked_in_schemas_match_the_schemas_of_the_three_documents() {
    for (generated, checked_in, name) in [
        (
            document_schema(COMPLETION_INSTALL),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schemas/completion-install-v1.json"
            )),
            "completion-install-v1.json",
        ),
        (
            document_schema(COMPLETION_CHECK),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schemas/completion-check-v1.json"
            )),
            "completion-check-v1.json",
        ),
        (
            document_schema(COMPLETION_PRINT),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/schemas/completion-print-v1.json"
            )),
            "completion-print-v1.json",
        ),
    ] {
        let generated = serde_json::to_string_pretty(&generated).unwrap() + "\n";
        assert_eq!(
            checked_in, generated,
            "schemas/{name} differs from the generated schema; replace the file with the generated schema"
        );
    }
}
