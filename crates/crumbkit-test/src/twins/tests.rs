//! Tests for the twin check.
//!
//! Each test writes a `/bin/sh` script that stands in for a tool, as the
//! recovery check's tests do.

use clap::{Arg, Command};
use crumbkit::Code;
use crumbkit::help::Guide;

use super::check_twin;
use crate::examples::Examples;
use crate::recoveries::Failing;
use crate::temp::TempDir;
use crate::testing::panic_message;

mod coverage;
mod exits;
mod variable;

/// The code of a refused command line.
const USAGE: Code = crumbkit::codes::CLI_USAGE;

/// The guide of a tool that declares nothing.
const GUIDE: Guide = Guide::new();

/// Returns the command tree of the scripts: a global `-C` option that
/// takes a directory, and a `books` group with the commands `list`, `show`,
/// `put`, aliased `add`, `get` and `stuck`. Every twin test uses it.
pub(super) fn tree() -> Command {
    let leaf = |name: &'static str| Command::new(name).arg(Arg::new("limit").long("limit"));
    Command::new("tool")
        .arg(Arg::new("dir").short('C').global(true))
        .subcommand(
            Command::new("books")
                .subcommand(leaf("list"))
                .subcommand(leaf("show"))
                .subcommand(leaf("put").alias("add"))
                .subcommand(leaf("get"))
                .subcommand(leaf("stuck")),
        )
}

/// The script's answers: `books list --limit 0` fails with the usage code in
/// text and in JSON, `--limit -1` fails with the usage code in the text of
/// `LineForm`, `--limit 1` succeeds, and `--limit 2` exits with code 0 as
/// well. Any other command line fails with code 3.
const SCRIPT: &str = r#"#!/bin/sh
case "$*" in
  'books list --limit 0') printf 'error: 0 is not a limit\n  help:  run `tool books list -h`\n  code:  tool::cli::usage\n' >&2; exit 2;;
  'books list --limit 0 --json') printf '%s\n' '{"schema":"tool/failure/v1","code":"tool::cli::usage","message":"m","recovery":{"kind":"retry"}}'; exit 2;;
  'books list --limit -1') printf 'ERR tool::cli::usage\n' >&2; exit 2;;
  'books list --limit 1'|'books list --limit 1 --json'|'books list --limit 2') echo listed; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// Writes the script and returns its directory and path.
fn tool(label: &str) -> (TempDir, String) {
    let dir = TempDir::new(label).unwrap();
    dir.write_script("tool", SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    (dir, path)
}

#[test]
fn a_twin_passes_when_the_refused_line_fails_with_its_code_and_the_admitted_line_succeeds() {
    let (_dir, path) = tool("twins-pass");
    let examples = Examples::new(&path, "");

    let text = check_twin(
        &tree(),
        &GUIDE,
        &examples,
        &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
        &["books", "list", "--limit", "1"],
    );
    let json = check_twin(
        &tree(),
        &GUIDE,
        &examples,
        &Failing::new(&USAGE, &["books", "list", "--limit", "0", "--json"]),
        &["books", "list", "--limit", "1", "--json"],
    );

    assert!(String::from_utf8_lossy(&text.stderr).starts_with("error: 0 is not a limit\n"));
    assert!(String::from_utf8_lossy(&json.stdout).contains("\"code\":\"tool::cli::usage\""));
}

#[test]
fn a_refused_line_that_succeeds_or_fails_with_another_code_fails_the_check() {
    const OTHER: Code = Code::new("tool", "books", "not_found");
    let (_dir, path) = tool("twins-refused");
    let examples = Examples::new(&path, "");

    let succeeds = panic_message(|| {
        check_twin(
            &tree(),
            &GUIDE,
            &examples,
            &Failing::new(&USAGE, &["books", "list", "--limit", "2"]),
            &["books", "list", "--limit", "1"],
        );
    });
    let other = panic_message(|| {
        check_twin(
            &tree(),
            &GUIDE,
            &examples,
            &Failing::new(&OTHER, &["books", "list", "--limit", "0"]),
            &["books", "list", "--limit", "1"],
        );
    });

    assert_eq!(
        succeeds.unwrap(),
        "the refused line `tool books list --limit 2` must fail with tool::cli::usage, and it exits with code 0"
    );
    assert_eq!(
        other.unwrap(),
        "the refused line `tool books list --limit 0` must fail with tool::books::not_found, and it fails with tool::cli::usage"
    );
}

#[test]
fn an_admitted_line_that_fails_fails_the_check() {
    let (_dir, path) = tool("twins-admitted");

    let message = panic_message(|| {
        check_twin(
            &tree(),
            &GUIDE,
            &Examples::new(&path, ""),
            &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
            &["books", "list", "--limit", "3"],
        );
    });

    assert_eq!(
        message.unwrap(),
        "the admitted line `tool books list --limit 3` must exit with code 0, and it ends with exit status: 3 after writing this to standard error:\nunexpected: books list --limit 3\n"
    );
}

#[test]
fn twins_that_differ_by_more_than_one_word_fail_the_check() {
    let (_dir, path) = tool("twins-words");

    let message = panic_message(|| {
        check_twin(
            &tree(),
            &GUIDE,
            &Examples::new(&path, ""),
            &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
            &["books", "list"],
        );
    });

    assert_eq!(
        message.unwrap(),
        "the refused line `tool books list --limit 0` and the admitted line `tool books list` must differ by one word added, removed or replaced"
    );
}

#[test]
fn a_refused_line_whose_code_the_check_cannot_read_fails_the_check_naming_the_sources_it_reads() {
    let (_dir, path) = tool("twins-unread");

    let message = panic_message(|| {
        check_twin(
            &tree(),
            &GUIDE,
            &Examples::new(&path, ""),
            &Failing::new(&USAGE, &["books", "list", "--limit", "-1"]),
            &["books", "list", "--limit", "1"],
        );
    });

    assert_eq!(
        message.unwrap(),
        "the refused line `tool books list --limit -1` must fail with tool::cli::usage, and it ends with exit status: 2 and prints no code that the check can read. The check reads a code from crumbkit's failure document on standard output or from a `code:` line on standard error"
    );
}

#[test]
fn a_failing_line_without_a_dir_runs_in_the_current_dir_of_examples() {
    let (dir, path) = tool("twins-current-dir");
    dir.write_script(
        "picky",
        "#!/bin/sh\ntest -f marker || exit 3\nexec \"$(dirname \"$0\")/tool\" \"$@\"\n",
    );
    let work = TempDir::new("twins-current-dir-work").unwrap();
    work.write("marker", "");
    let picky = path.replace("/tool", "/picky");
    let work_path = work.path().display().to_string();

    check_twin(
        &tree(),
        &GUIDE,
        &Examples::new(&picky, "").current_dir(&work_path),
        &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
        &["books", "list", "--limit", "1"],
    );
}

#[test]
fn both_lines_of_a_twin_run_in_a_copy_of_the_refused_line_s_xdg_fixture() {
    let (dir, path) = tool("twins-xdg");
    dir.write_script(
        "plugged",
        "#!/bin/sh\ntest -f \"$XDG_CONFIG_HOME/tool/plugin\" || exit 3\nexec \"$(dirname \"$0\")/tool\" \"$@\"\n",
    );
    let fixture = TempDir::new("twins-xdg-fixture").unwrap();
    fixture.create_dir("config/tool").unwrap();
    fixture.write("config/tool/plugin", "");
    let plugged = path.replace("/tool", "/plugged");
    let fixture_path = fixture.path().display().to_string();

    check_twin(
        &tree(),
        &GUIDE,
        &Examples::new(&plugged, ""),
        &Failing::new(&USAGE, &["books", "list", "--limit", "0"]).xdg(&fixture_path),
        &["books", "list", "--limit", "1"],
    );
}

#[test]
fn an_admitted_line_that_only_adds_help_or_version_fails_the_check() {
    let (_dir, path) = tool("twins-help");
    let examples = Examples::new(&path, "");

    for word in ["-h", "--help", "-V", "--version"] {
        let message = panic_message(|| {
            check_twin(
                &tree(),
                &GUIDE,
                &examples,
                &Failing::new(&USAGE, &["books", "list", "--limit", "0"]),
                &["books", "list", "--limit", "0", word],
            );
        });

        assert_eq!(
            message.unwrap(),
            format!(
                "the admitted line `tool books list --limit 0 {word}` adds `{word}`, which makes the tool answer without running the command, so the twin proves nothing about the refused line `tool books list --limit 0`. A code that no word of a command line causes needs an opt-out made with Allow::untwinned"
            )
        );
    }
    let help = panic_message(|| {
        check_twin(
            &tree(),
            &GUIDE,
            &examples,
            &Failing::new(&USAGE, &["books", "list"]),
            &["help", "books", "list"],
        );
    });

    assert!(help.unwrap().contains("adds `help`"));
}

#[test]
fn a_twin_whose_admitted_line_runs_another_command_fails_the_check_whatever_leaves_the_guide_has() {
    let (_dir, path) = tool("twins-leaf");
    let examples = Examples::new(&path, "");
    let checked = |refused: &'static [&'static str], admitted: &[&str]| {
        panic_message(|| {
            check_twin(
                &tree(),
                &GUIDE,
                &examples,
                &Failing::new(&USAGE, refused),
                admitted,
            );
        })
    };

    assert_eq!(
        checked(&["books", "put", "7"], &["books", "get", "7"]).unwrap(),
        "the refused line `tool books put 7` runs the command `books put`, and the admitted line `tool books get 7` runs `books get`, so the twin proves nothing about the refused command. A code that no word of the command causes needs an opt-out made with Allow::untwinned"
    );
    assert_eq!(
        checked(&["books", "put", "7"], &["books", "pt", "7"]).unwrap(),
        "the refused line `tool books put 7` runs the command `books put`, and the admitted line `tool books pt 7` runs no command, so the twin proves nothing about the refused command. A code that no word of the command causes needs an opt-out made with Allow::untwinned"
    );
    // A refused line that runs no command, such as a misspelt command,
    // keeps its twin. The run of this one fails, because the script does
    // not know it.
    assert!(
        checked(&["books", "pt", "7"], &["books", "put", "7"])
            .unwrap()
            .starts_with("the refused line `tool books pt 7` must fail with tool::cli::usage")
    );
}

#[test]
fn a_twin_whose_refused_line_runs_no_command_fails_when_its_admitted_line_runs_none_either() {
    let (_dir, path) = tool("twins-no-command");
    let examples = Examples::new(&path, "");
    let checked = |refused: &'static [&'static str], admitted: &[&str]| {
        panic_message(|| {
            check_twin(
                &tree(),
                &GUIDE,
                &examples,
                &Failing::new(&USAGE, refused),
                admitted,
            );
        })
    };

    assert_eq!(
        checked(&["books", "lst"], &["books"]).unwrap(),
        "the refused line `tool books lst` runs no command, and the admitted line `tool books` runs none either, so the twin proves nothing about a command. A twin of a line that runs no command admits a line that runs one"
    );
    // `books list` runs a command, so the twin keeps its rule. The run
    // fails, because the script does not know the refused line.
    assert!(
        checked(&["books", "lst"], &["books", "list"])
            .unwrap()
            .starts_with("the refused line `tool books lst` must fail with tool::cli::usage")
    );
}

#[test]
fn a_twin_with_a_global_option_before_the_path_runs_the_command_after_the_option_s_value() {
    let (_dir, path) = tool("twins-global");
    let examples = Examples::new(&path, "");
    let checked = |refused: &'static [&'static str], admitted: &[&str]| {
        panic_message(|| {
            check_twin(
                &tree(),
                &GUIDE,
                &examples,
                &Failing::new(&USAGE, refused),
                admitted,
            );
        })
    };

    assert!(
        checked(&["-C", "books", "books", "put", "7"], &["-C", "books", "books", "get", "7"])
            .unwrap()
            .starts_with("the refused line `tool -C books books put 7` runs the command `books put`, and the admitted line `tool -C books books get 7` runs `books get`")
    );
    // The alias `add` runs `books put`, so the twin keeps its command. The
    // run fails, because the script does not know the line.
    assert!(
        checked(
            &["-C", "empty", "books", "put", "7"],
            &["-C", "empty", "books", "add", "7"]
        )
        .unwrap()
        .starts_with(
            "the refused line `tool -C empty books put 7` must fail with tool::cli::usage"
        )
    );
}

/// A guide that spells codes with their segments joined by `.`.
const DOTTED: Guide = Guide::new().spelling(crumbkit::Spelling::custom(|code, f| {
    write!(f, "{}.{}.{}", code.component(), code.area(), code.reason())
}));

/// The answers of a tool whose guide sets the dotted spelling: `books get
/// --limit 0` fails with the usage code in that spelling, `books list
/// --limit 0` fails with it in the colon spelling, and `--limit 1` succeeds
/// for both. Any other command line fails with code 3.
const DOTTED_SCRIPT: &str = r#"#!/bin/sh
case "$*" in
  'books get --limit 0') printf 'error: 0 is not a limit\n  code:  tool.cli.usage\n' >&2; exit 2;;
  'books list --limit 0') printf 'error: 0 is not a limit\n  code:  tool::cli::usage\n' >&2; exit 2;;
  'books get --limit 1'|'books list --limit 1') echo listed; exit 0;;
  *) echo "unexpected: $*" >&2; exit 3;;
esac
"#;

/// Runs `check_twin` with the dotted guide on the dotted script, refusing
/// `books <verb> --limit 0` and admitting `--limit 1`, and returns the
/// panic message, or `None` when it passes.
fn dotted_twin(verb: &str) -> Option<String> {
    let dir = TempDir::new("twins-dotted").unwrap();
    dir.write_script("tool", DOTTED_SCRIPT);
    let path = dir.path().join("tool").display().to_string();
    let examples = Examples::new(&path, "");
    panic_message(|| {
        check_twin(
            &tree(),
            &DOTTED,
            &examples,
            &Failing::new(&USAGE, &["books", verb, "--limit", "0"]),
            &["books", verb, "--limit", "1"],
        );
    })
}

#[test]
fn a_twin_passes_when_the_refused_line_prints_its_code_in_the_guides_custom_spelling() {
    assert_eq!(dotted_twin("get"), None);
}

#[test]
fn a_twin_fails_when_the_refused_line_prints_its_code_in_colons_and_the_guide_sets_another_spelling()
 {
    let message = dotted_twin("list").unwrap();

    assert!(message.contains("tool.cli.usage"), "{message}");
    assert!(message.contains("tool::cli::usage"), "{message}");
}
