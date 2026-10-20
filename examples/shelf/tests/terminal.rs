//! The terminal test of shelf, which runs each example with standard input,
//! standard output and standard error on pseudo-terminals and compares the
//! bytes with the example's golden files, which a piped run wrote: the
//! `.txt` file for standard output and the `.stderr` file, when the example
//! has one, for standard error. A result prints the same bytes at a
//! terminal as through a pipe (ADR-0049, OUTPUT-1), except that a text
//! table fits the width of a narrow terminal (ADR-0121).
//!
//! The test opens the pseudo-terminals with rustix's safe functions and runs
//! on macOS and Linux only. Each terminal is in raw mode, so the terminal
//! passes the bytes through without turning a newline into a carriage return
//! and a newline.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::fs::File;
use std::io::{self, ErrorKind, Read as _};
use std::os::fd::OwnedFd;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use rustix::fs::{Mode, OFlags};
use rustix::io::{Errno, FdFlags, fcntl_setfd};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{OptionalActions, Winsize, tcgetattr, tcsetattr, tcsetwinsize};
use shelf::GUIDE;
use tempfile::TempDir;

/// The longest a run may take before the test stops it. A run that asks a
/// question waits for an answer that never comes, so the limit turns a hang
/// into a failure.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The terminal type each run sees. clap colours help only when `TERM`
/// names a terminal that supports colour.
const TERM: &str = "xterm-256color";

/// The directory that holds shelf's example golden files.
const GOLDEN: &str = "tests/golden/examples";

/// The width in columns of a terminal wider than any table of shelf's
/// examples, so no table is fitted at it and each run prints the bytes of
/// its piped golden file.
const WIDE: u16 = 500;

/// One pseudo-terminal: the side the test reads and the side the run
/// writes to.
struct Pty {
    /// The side that the test reads the run's bytes from.
    main: OwnedFd,
    /// The terminal that the run receives as a standard stream.
    terminal: OwnedFd,
}

/// Opens a pseudo-terminal in raw mode, `columns` wide, with both sides
/// closed in any program that the test starts.
fn pty(columns: u16) -> io::Result<Pty> {
    let main = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    fcntl_setfd(&main, FdFlags::CLOEXEC)?;
    grantpt(&main)?;
    unlockpt(&main)?;
    let name = ptsname(&main, Vec::new())?;
    let terminal = rustix::fs::open(
        name.as_c_str(),
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    let mut termios = tcgetattr(&terminal)?;
    termios.make_raw();
    tcsetattr(&terminal, OptionalActions::Now, &termios)?;
    let size = Winsize {
        ws_row: 24,
        ws_col: columns,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    tcsetwinsize(&terminal, size)?;
    Ok(Pty { main, terminal })
}

/// Reads everything a run writes to `main` on a thread of its own, so a run
/// that fills one terminal's buffer does not wait on the test. Reading ends
/// when every copy of the terminal side is closed, which Linux reports as
/// an `EIO` error and macOS as the end of the file.
fn drain(main: OwnedFd) -> JoinHandle<io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut file = File::from(main);
        let mut bytes = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => return Ok(bytes),
                Ok(read) => bytes.extend_from_slice(&buffer[..read]),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.raw_os_error() == Some(Errno::IO.raw_os_error()) => {
                    return Ok(bytes);
                }
                Err(error) => return Err(error),
            }
        }
    })
}

/// Returns the bytes that the thread of [`drain`] read.
fn collect(reader: JoinHandle<io::Result<Vec<u8>>>) -> io::Result<Vec<u8>> {
    reader
        .join()
        .map_err(|_| io::Error::other("the thread that reads a pseudo-terminal panicked"))?
}

/// What one run printed on standard output and standard error, and how it
/// ended.
struct Run {
    /// The bytes on standard output.
    stdout: Vec<u8>,
    /// The bytes on standard error.
    stderr: Vec<u8>,
    /// How the run ended.
    status: ExitStatus,
}

/// Runs the built `shelf` binary with `args`, standard input and standard
/// output on one pseudo-terminal `columns` wide, standard error on another, a cleared
/// environment with `TERM` and XDG directories in a fresh temporary
/// directory, and returns what it printed. A run that does not end within
/// [`TIMEOUT`] is killed and returns a `TimedOut` error.
fn shelf(args: &[&str], columns: u16) -> io::Result<Run> {
    let home = TempDir::new()?;
    let out = pty(columns)?;
    let err = pty(columns)?;
    let input = out.terminal.try_clone()?;
    #[expect(
        clippy::disallowed_methods,
        reason = "the terminal test runs the built binary with its standard streams on pseudo-terminals, which a test command helper cannot set"
    )]
    let mut command = Command::new(env!("CARGO_BIN_EXE_shelf"));
    command
        .args(args)
        .env_clear()
        .env("TERM", TERM)
        .stdin(Stdio::from(input))
        .stdout(Stdio::from(out.terminal))
        .stderr(Stdio::from(err.terminal));
    for (variable, dir) in [
        ("XDG_CONFIG_HOME", "config"),
        ("XDG_DATA_HOME", "data"),
        ("XDG_STATE_HOME", "state"),
        ("XDG_CACHE_HOME", "cache"),
    ] {
        command.env(variable, home.path().join(dir));
    }
    let mut child = command.spawn()?;
    // The command holds the test's copies of the terminal sides, and
    // dropping it closes them, so reading ends when the run exits.
    drop(command);
    let stdout = drain(out.main);
    let stderr = drain(err.main);
    let status = wait(&mut child)?;
    Ok(Run {
        stdout: collect(stdout)?,
        stderr: collect(stderr)?,
        status,
    })
}

/// Waits for `child` to exit and returns how it ended. A run that does not
/// end within [`TIMEOUT`] is killed and returns a `TimedOut` error.
fn wait(child: &mut Child) -> io::Result<ExitStatus> {
    let started = Instant::now();
    while started.elapsed() < TIMEOUT {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        thread::sleep(Duration::from_millis(10));
    }
    child.kill()?;
    Err(io::Error::new(
        ErrorKind::TimedOut,
        format!("the run did not end within {} seconds", TIMEOUT.as_secs()),
    ))
}

#[test]
fn help_at_a_terminal_has_colour_so_the_test_reaches_a_real_terminal() {
    let run = shelf(&["--help"], WIDE).unwrap();
    assert!(
        run.status.success(),
        "shelf --help ends with {}",
        run.status
    );
    assert!(
        run.stdout.windows(2).any(|pair| pair == b"\x1b["),
        "help at a terminal has no colour, so shelf does not see a terminal: {}",
        String::from_utf8_lossy(&run.stdout)
    );
}

#[test]
fn every_example_prints_the_same_bytes_at_a_terminal_as_through_a_pipe() {
    let mut compared = 0;
    for leaf in GUIDE.get_leaves() {
        let path = leaf.path();
        for example in leaf.get_examples() {
            let mut line = std::iter::once("shelf")
                .chain(path.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            for &word in example.words() {
                line.push(' ');
                line.push_str(&crumbkit::help::quote(word));
            }
            let golden = format!("{GOLDEN}/{}", crumbkit_test::example_golden_name(&line));
            // The example check fails an example without a golden file
            // unless shelf opts the example out, so a missing file is an
            // example that the example check skips too.
            let Ok(expected) = std::fs::read(&golden) else {
                continue;
            };
            let args: Vec<&str> = path.iter().chain(example.words()).copied().collect();
            // A table narrower than the terminal prints unchanged, so at a
            // terminal wider than every table each run prints the bytes of
            // its piped golden file (ADR-0121).
            let run = shelf(&args, WIDE).unwrap_or_else(|error| panic!("{line}: {error}"));
            assert!(
                run.status.success(),
                "{line} ends with {} at a terminal: {}",
                run.status,
                String::from_utf8_lossy(&run.stderr)
            );
            assert!(
                run.stdout == expected,
                "{line} prints different bytes at a terminal than its golden file {golden} holds:\n{}",
                String::from_utf8_lossy(&run.stdout)
            );
            // An example that writes to standard error has a `.stderr`
            // golden file beside its `.txt` one, and an example without one
            // writes nothing there.
            let stderr_golden = golden.replace(".txt", ".stderr");
            let expected_stderr = std::fs::read(&stderr_golden).unwrap_or_default();
            assert!(
                run.stderr == expected_stderr,
                "{line} writes different bytes to standard error at a terminal than {stderr_golden} holds:\n{}",
                String::from_utf8_lossy(&run.stderr)
            );
            compared += 1;
        }
    }
    assert!(compared > 0, "the test compares no example");
}

#[test]
fn a_table_at_a_narrow_terminal_cuts_its_widest_column_and_keeps_each_id_whole() {
    let run = shelf(&["books", "list", "--all"], 40).unwrap();

    assert!(
        run.status.success(),
        "shelf books list --all ends with {} at a terminal: {}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    // The piped table is 63 characters wide. The title is its widest
    // column, so it narrows by the excess of 23 to 10 characters, and each
    // longer title ends with `…`.
    let expected = "\
ID  TITLE       AUTHOR              YEAR
1   Pride and…  Jane Austen         1813
2   Frankenst…  Mary Shelley        1818
3   Moby-Dick   Herman Melville     1851
4   Great Exp…  Charles Dickens     1861
5   Middlemar…  George Eliot        1871
6   The Adven…  Arthur Conan Doyle  1892
";
    assert_eq!(String::from_utf8_lossy(&run.stdout), expected);
}
