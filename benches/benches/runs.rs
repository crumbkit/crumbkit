//! crumbkit's benchmarks, which run shelf's per-run paths in-process and
//! count their instructions with Callgrind and their allocations with DHAT
//! (ADR-0008, PERF-1).
//!
//! Each benchmark calls `shelf::run_from`, which takes the path that every run
//! of the `shelf` binary takes: the alias lookup, `crumbkit::parse_from` and
//! `crumbkit::finish`. Its output goes to the
//! benchmark process's standard output, which gungraun reads through a pipe,
//! so help prints without colour. gungraun clears the environment of each
//! benchmark and the benchmarks set only the XDG directories, so a
//! benchmark never reads a person's aliases or writes to their usage log.
//!
//! Each benchmark has a limit on Callgrind's instruction count (`Ir`) and on
//! DHAT's count of allocated blocks, and `cargo bench` fails when a count goes
//! over its limit. The counts come from the nightly job on `ubuntu-24.04`,
//! and each limit is that count plus 10 percent, rounded up. A count from
//! another machine or architecture is not comparable with the limits.
#![expect(
    missing_docs,
    reason = "gungraun's macros generate modules, functions and constants without docs"
)]
#![expect(
    clippy::disallowed_methods,
    reason = "gungraun's `main!` exits the process with the runner's exit code, and the \
              logged benchmark's setup owns its temporary state directory"
)]

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use gungraun::{
    Callgrind, Dhat, DhatMetric, EventKind, LibraryBenchmarkConfig, library_benchmark,
    library_benchmark_group, main,
};

/// The directory that holds the XDG directories of every benchmark. cargo
/// creates it for benchmarks inside the target directory.
const XDG_ROOT: &str = concat!(env!("CARGO_TARGET_TMPDIR"), "/xdg");

/// The XDG base directory variables that each benchmark sets, with the name
/// of the directory under [`XDG_ROOT`] that each one points at.
const XDG: [(&str, &str); 4] = [
    ("XDG_CONFIG_HOME", "config"),
    ("XDG_DATA_HOME", "data"),
    ("XDG_STATE_HOME", "state"),
    ("XDG_CACHE_HOME", "cache"),
];

/// The limits of one benchmark.
struct Limits {
    /// The most instructions that Callgrind may count.
    instructions: u64,
    /// The most blocks that DHAT may count as allocated.
    allocations: u64,
}

// Each limit's comment gives the count that it comes from. The counts come
// from runs of `scheduled.yml` on `ubuntu-24.04`. Run 38191991177 gave the
// instruction counts of `books list` with the usage log, of `commands list`
// and of the refusal. Run 38163868931 gave the instruction counts of
// `shelf -h` and `books list -h`. Run 38215644933 gave every block count,
// with DHAT recording 500 stack frames. Run 40855949287 gave the
// instruction count of `books list` without the usage log, after the
// listing's next steps, summary text and default rows had added to it.

/// The limits of `shelf books list`.
const BOOKS_LIST: Limits = Limits {
    // The count was 716207 instructions.
    instructions: 787_828,
    // The count was 1142 blocks.
    allocations: 1257,
};

/// The limits of `shelf -h`.
const ROOT_HELP: Limits = Limits {
    // The count was 350802 instructions.
    instructions: 385_883,
    // The count was 413 blocks.
    allocations: 455,
};

/// The limits of `shelf books list -h`.
const LEAF_HELP: Limits = Limits {
    // The count was 447139 instructions.
    instructions: 491_853,
    // The count was 628 blocks.
    allocations: 691,
};

/// The limits of `shelf commands list --all --json --full`.
const COMMANDS_LIST: Limits = Limits {
    // The count was 2581595 instructions.
    instructions: 2_839_755,
    // The count was 3377 blocks.
    allocations: 3715,
};

/// The limits of `shelf books lst`, which clap refuses.
const REFUSAL: Limits = Limits {
    // The count was 417385 instructions.
    instructions: 459_124,
    // The count was 539 blocks.
    allocations: 593,
};

/// The limits of `shelf books list` with the usage log started.
const BOOKS_LIST_LOGGED: Limits = Limits {
    // The count was 793688 instructions.
    instructions: 873_057,
    // The count was 1203 blocks.
    allocations: 1324,
};

/// The stack depth that valgrind records for each allocation, which is
/// valgrind's largest. gungraun counts an allocation only when its recorded
/// stack reaches the benchmark function, and valgrind's default of 12 frames
/// left out allocations deeper than that, so a change in inlining moved the
/// block count with no change in the allocations made.
const NUM_CALLERS: &str = "num-callers=500";

/// Returns the configuration that runs Callgrind and DHAT with `limits` and
/// sets the XDG directories.
fn config(limits: &Limits) -> LibraryBenchmarkConfig {
    let mut config = LibraryBenchmarkConfig::default();
    config
        .tool(Callgrind::default().hard_limits([(EventKind::Ir, limits.instructions)]))
        .tool(
            Dhat::with_args([NUM_CALLERS])
                .hard_limits([(DhatMetric::TotalBlocks, limits.allocations)]),
        );
    for (variable, name) in XDG {
        config.env(variable, format!("{XDG_ROOT}/{name}"));
    }
    config
}

// Runs shelf on `words` after the program name, as the `shelf` binary runs
// without the usage log. gungraun's macro accepts no doc comment here.
#[library_benchmark]
#[bench::books_list(args = (&["books", "list"]), config = config(&BOOKS_LIST))]
#[bench::root_help(args = (&["-h"]), config = config(&ROOT_HELP))]
#[bench::leaf_help(args = (&["books", "list", "-h"]), config = config(&LEAF_HELP))]
#[bench::commands_list(
    args = (&["commands", "list", "--all", "--json", "--full"]),
    config = config(&COMMANDS_LIST)
)]
#[bench::refusal(args = (&["books", "lst"]), config = config(&REFUSAL))]
fn run(words: &[&str]) -> ExitCode {
    shelf::run_from(std::iter::once("shelf").chain(words.iter().copied()))
}

// Starts the usage log, runs `shelf books list` and writes the run's line,
// as the `shelf` binary does. The line is written when the guard drops,
// which happens inside the measured function. gungraun evaluates the
// arguments before it starts to count, so the setup is not measured.
#[library_benchmark]
#[bench::books_list(args = (state_with_key()), config = config(&BOOKS_LIST_LOGGED))]
fn logged(_: PathBuf) -> ExitCode {
    let log = crumbkit_usage::start();
    let code = shelf::run_from(["shelf", "books", "list"]);
    drop(log);
    code
}

/// Empties the state directory and gives shelf a usage log key, so the
/// benchmark measures a run whose key already exists, as every run after a
/// tool's first does, and whose log holds no lines. Returns the directory
/// that holds the log.
fn state_with_key() -> PathBuf {
    let shelf = PathBuf::from(format!("{XDG_ROOT}/state/shelf"));
    // The directory does not exist before the first run, so the setup
    // ignores a failed removal.
    let _ = fs::remove_dir_all(&shelf);
    let created = fs::create_dir_all(&shelf);
    // The key's bytes need not be random, because the benchmark only
    // measures how the log uses the key.
    let written = created.and_then(|()| fs::write(shelf.join("usage.key"), [7_u8; 16]));
    if let Err(error) = written {
        // gungraun reports a setup that panics as a failed benchmark, which
        // is the right outcome when the setup cannot prepare the log.
        #[expect(
            clippy::panic,
            reason = "a benchmark without its key file would measure the wrong path"
        )]
        {
            panic!(
                "cannot prepare the usage log state in {}: {error}",
                shelf.display()
            );
        }
    }
    shelf
}

library_benchmark_group!(name = runs, benchmarks = [run, logged]);

main!(library_benchmark_groups = runs);
