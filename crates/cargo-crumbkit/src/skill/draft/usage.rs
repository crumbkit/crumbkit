//! The exported usage that `skill draft --usage` reads, and the ranking of
//! its commands for the most-used section (ADR-0194).

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::tool::names;

#[cfg(test)]
mod tests;

/// One run of a tool, from one record of an exported usage log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Run {
    /// The run's span id, or `None` for a record without one.
    pub(crate) id: Option<String>,
    /// The name of the tool's root command, such as `shelf`.
    pub(crate) tool: String,
    /// The keyed hash of the run's session.
    pub(crate) session: String,
    /// When the run ended, in RFC 3339 with milliseconds, which sorts as
    /// text.
    pub(crate) at: String,
    /// The command path below the tool's name, such as `books list`, or
    /// `None` for a run of the root.
    pub(crate) command: Option<String>,
    /// The flags typed on the command line, by their long names.
    pub(crate) flags: Vec<String>,
    /// Whether the run exited with a code other than 0.
    pub(crate) failed: bool,
    /// Whether the run printed a help page.
    pub(crate) help: bool,
}

/// One command of the most-used section, with its figures.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Ranked {
    /// The command path below the tool's name, such as `books list`.
    pub(crate) command: String,
    /// The runs of the command.
    pub(crate) runs: u64,
    /// The runs that failed, or that ran right after a failed run of the
    /// same command in the same session.
    pub(crate) wasted: u64,
    /// The flags of the runs, the most used first and equal uses by name.
    pub(crate) flags: Vec<String>,
}

/// The runs that a usage directory holds.
#[derive(Debug, Default)]
pub(crate) struct Usage {
    /// Every run, each span id once.
    pub(crate) runs: Vec<Run>,
    /// The non-empty lines that are not an exported usage record.
    pub(crate) unreadable: u64,
}

/// One line of the `<tool>/self-runs-export/v1` stream, with the fields
/// that the ranking reads. serde ignores the other fields.
#[derive(Deserialize)]
struct Line {
    /// The line's document id.
    schema: String,
    /// The line's data.
    data: Data,
}

/// The data of one exported line.
#[derive(Deserialize)]
struct Data {
    /// The stored usage record.
    record: Record,
}

/// The fields of a stored `<tool>/usage/v1` record that the ranking reads.
#[derive(Deserialize)]
struct Record {
    /// The tool's name.
    tool: String,
    /// When the run ended.
    at: String,
    /// The keyed hash of the run's session.
    session: String,
    /// The run's span id.
    #[serde(default)]
    run: Option<String>,
    /// The command path below the tool's name.
    #[serde(default)]
    command: Option<String>,
    /// The flags typed on the command line.
    #[serde(default)]
    flags: Vec<String>,
    /// The run's exit code.
    #[serde(default)]
    exit: Option<u8>,
    /// The help page that the run printed.
    #[serde(default)]
    help: Option<String>,
}

/// Returns the run that `line`, one line of `self runs export --json`,
/// holds, or `None` when the line is not an exported usage record.
pub(crate) fn parse(line: &str) -> Option<Run> {
    let line: Line = serde_json::from_str(line).ok()?;
    if !names(&line.schema, &crumbkit_usage::SELF_RUNS_EXPORT) {
        return None;
    }
    let record = line.data.record;
    Some(Run {
        id: record.run,
        tool: record.tool,
        session: record.session,
        at: record.at,
        command: record.command,
        flags: record.flags,
        failed: record.exit.is_some_and(|exit| exit != 0),
        help: record.help.is_some(),
    })
}

/// Reads every `.jsonl` file under `dir` and in its subdirectories, in path
/// order, and returns their runs with each span id once, because two
/// exports may hold the same run. It returns `None` when `dir` does not
/// exist.
///
/// # Errors
///
/// Returns the path and the I/O error of a directory or file that exists
/// but cannot be read.
pub(crate) fn read(dir: &Path) -> Result<Option<Usage>, (PathBuf, io::Error)> {
    if !dir.exists() {
        return Ok(None);
    }
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    let mut usage = Usage::default();
    let mut seen = HashSet::new();
    for file in files {
        let text = fs::read_to_string(&file).map_err(|error| (file.clone(), error))?;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            match parse(line) {
                Some(run) if run.id.as_ref().is_none_or(|id| seen.insert(id.clone())) => {
                    usage.runs.push(run);
                }
                Some(_) => {}
                None => usage.unreadable += 1,
            }
        }
    }
    Ok(Some(usage))
}

/// Adds the `.jsonl` files under `dir` and its subdirectories to `files`.
fn collect(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), (PathBuf, io::Error)> {
    let failed = |error| (dir.to_path_buf(), error);
    for entry in fs::read_dir(dir).map_err(failed)? {
        let path = entry.map_err(failed)?.path();
        if path.is_dir() {
            collect(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "jsonl")
        {
            files.push(path);
        }
    }
    Ok(())
}

/// Returns the commands of `runs`, the most-used first.
///
/// A command's score is its runs × (1 + the share of its runs that were
/// wasted), which is its runs plus its wasted runs. A run is wasted when it
/// failed, or when it is a retry: the run before it in its session, by time,
/// ran the same command and failed. Runs that print a help page and runs of
/// the root are not ranked and are not counted as the run before another.
/// Equal scores rank by command.
pub(crate) fn rank(runs: &[Run]) -> Vec<Ranked> {
    let mut sessions: BTreeMap<&str, Vec<&Run>> = BTreeMap::new();
    for run in runs.iter().filter(|run| !run.help && run.command.is_some()) {
        sessions.entry(run.session.as_str()).or_default().push(run);
    }
    let mut tally: BTreeMap<&str, (u64, u64, BTreeMap<&str, u64>)> = BTreeMap::new();
    for session in sessions.values_mut() {
        session.sort_by(|a, b| a.at.cmp(&b.at));
        let mut before: Option<&Run> = None;
        for &run in session.iter() {
            let retry = before.is_some_and(|before| before.failed && before.command == run.command);
            let command = run.command.as_deref().unwrap_or_default();
            let (count, wasted, flags) = tally.entry(command).or_default();
            *count += 1;
            *wasted += u64::from(run.failed || retry);
            for flag in &run.flags {
                *flags.entry(flag.as_str()).or_default() += 1;
            }
            before = Some(run);
        }
    }
    let mut ranked: Vec<Ranked> = tally
        .into_iter()
        .map(|(command, (runs, wasted, flags))| {
            let mut flags: Vec<(&str, u64)> = flags.into_iter().collect();
            flags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
            Ranked {
                command: command.to_owned(),
                runs,
                wasted,
                flags: flags.into_iter().map(|(flag, _)| flag.to_owned()).collect(),
            }
        })
        .collect();
    ranked.sort_by(|a, b| {
        (b.runs + b.wasted)
            .cmp(&(a.runs + a.wasted))
            .then_with(|| a.command.cmp(&b.command))
    });
    ranked
}
