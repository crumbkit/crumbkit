//! Tests for `requested`, the rule that the usage log shares, and for
//! `complete_to`.

use std::ffi::OsStr;

use super::requested;

#[test]
fn a_completion_variable_that_is_unset_empty_or_zero_is_no_request() {
    assert!(!requested(None));
    assert!(!requested(Some(OsStr::new(""))));
    assert!(!requested(Some(OsStr::new("0"))));
    assert!(requested(Some(OsStr::new("fish"))));
}

#[cfg(feature = "complete")]
mod answers {
    use std::ffi::OsStr;
    use std::io::{self, Write};
    use std::process::ExitCode;

    use clap::Command;

    use super::super::{VAR, complete_to};
    use crate::Sink;

    /// A tool named shelf with `books list`, which has the hidden alias
    /// `ls`, `books show` and `books remove`.
    fn tool() -> Command {
        Command::new("shelf").subcommand(
            Command::new("books")
                .subcommand(Command::new("list").alias("ls"))
                .subcommand(Command::new("show"))
                .subcommand(Command::new("remove")),
        )
    }

    /// Completes `words` against `command` for `shell`, as the shell runs
    /// `shelf -- <words>` with `COMPLETE` set, and returns the exit code,
    /// standard output and standard error.
    fn complete(command: Command, shell: &str, words: &[&str]) -> (ExitCode, String, String) {
        complete_with(command, VAR, shell, words)
    }

    /// Completes `words` as [`complete`] does, for a tool whose completion
    /// variable is `variable`.
    fn complete_with(
        command: Command,
        variable: &str,
        shell: &str,
        words: &[&str],
    ) -> (ExitCode, String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let args = ["shelf", "--"].iter().chain(words);
        let code = complete_to(
            &mut Sink::new(&mut out, &mut err),
            command,
            variable,
            OsStr::new(shell),
            args,
            None,
        );
        (
            code,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    /// Returns the first column of each line of fish's candidates.
    fn values(out: &str) -> Vec<&str> {
        out.lines()
            .map(|line| line.split('\t').next().unwrap())
            .collect()
    }

    #[test]
    fn completion_leaves_out_a_command_that_the_tool_hid_at_run_time() {
        let command = tool().mut_subcommand("books", |books| {
            books.mut_subcommand("remove", |remove| remove.hide(true))
        });

        let (code, out, err) = complete(command, "fish", &["shelf", "books", ""]);

        assert_eq!(code, ExitCode::SUCCESS);
        let values = values(&out);
        assert!(values.contains(&"show"), "{out}");
        assert!(!values.contains(&"remove"), "{out}");
        assert_eq!(err, "");
    }

    #[test]
    fn a_registration_request_writes_the_shells_script_with_the_tools_name() {
        let (code, out, err) = complete(tool(), "/usr/bin/fish", &[]);

        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.starts_with("complete "), "{out}");
        assert!(out.contains("--command shelf"), "{out}");
        assert!(out.contains("COMPLETE=fish"), "{out}");
        assert_eq!(err, "");
    }

    #[test]
    fn a_registration_request_names_the_completion_variable_that_the_tool_declared() {
        let (code, out, err) = complete_with(tool(), "SHELF_COMPLETE", "fish", &[]);

        assert_eq!(code, ExitCode::SUCCESS);
        assert!(out.contains("SHELF_COMPLETE=fish"), "{out}");
        assert!(!out.contains(" COMPLETE=fish"), "{out}");
        assert_eq!(err, "");
    }

    #[test]
    fn a_completion_for_an_unknown_shell_writes_nothing_and_returns_failure() {
        let (code, out, err) = complete(tool(), "tcsh", &["shelf", ""]);

        assert_eq!(code, ExitCode::FAILURE);
        assert_eq!(out, "");
        assert_eq!(err, "");
    }

    /// A writer that fails every write.
    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("the disk is full"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_completion_whose_write_fails_writes_nothing_to_standard_error() {
        let mut err = Vec::new();
        let args = ["shelf", "--", "shelf", ""];

        let code = complete_to(
            &mut Sink::new(Broken, &mut err),
            tool(),
            VAR,
            OsStr::new("fish"),
            args,
            None,
        );

        assert_eq!(code, ExitCode::FAILURE);
        assert_eq!(err, b"");
    }
}
