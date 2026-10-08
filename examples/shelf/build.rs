//! Sets `SHELF_VERSION`, the version that `shelf --version` prints.
//!
//! A build from a git checkout appends the commit as semver build metadata,
//! such as `0.0.0+git-951022a3a14a`, and adds `.dirty` when tracked files have
//! uncommitted changes. A build without git, or outside a checkout, prints the
//! plain package version.

use std::process::Command;

fn main() {
    let package_version = env!("CARGO_PKG_VERSION");
    let version = match git_metadata() {
        Some(metadata) => format!("{package_version}+{metadata}"),
        None => package_version.to_owned(),
    };
    tell_cargo(&format!("cargo::rustc-env=SHELF_VERSION={version}"));
}

/// Returns `git-<commit>`, with `.dirty` appended when tracked files differ
/// from the commit, or `None` when git cannot describe this checkout.
///
/// It also tells cargo to rerun this script when the commit, the index or
/// shelf's own sources change. An edit elsewhere in the repository changes the
/// dirty flag only at the next rerun, such as after `git add`.
fn git_metadata() -> Option<String> {
    let commit = git(&["rev-parse", "--short=12", "HEAD"])?;

    // `--git-path` resolves each path inside the git directory, which in a
    // worktree is not `.git/` at the repository root.
    for path in ["HEAD", "index", "packed-refs"] {
        tell_cargo(&format!(
            "cargo::rerun-if-changed={}",
            git(&["rev-parse", "--git-path", path])?
        ));
    }
    // On a branch, a new commit changes the branch's ref file and leaves HEAD
    // alone. A detached HEAD has no branch, and HEAD itself holds the commit.
    if let Some(branch) = git(&["symbolic-ref", "--quiet", "HEAD"]) {
        tell_cargo(&format!(
            "cargo::rerun-if-changed={}",
            git(&["rev-parse", "--git-path", &branch])?
        ));
    }
    tell_cargo("cargo::rerun-if-changed=src");

    // Untracked files do not make the build dirty, as with `git describe --dirty`.
    let changes = git(&["status", "--porcelain", "--untracked-files=no"])?;
    if changes.is_empty() {
        Some(format!("git-{commit}"))
    } else {
        Some(format!("git-{commit}.dirty"))
    }
}

/// Runs git with `args` and returns its trimmed output, or `None` when git is
/// missing or fails.
fn git(args: &[&str]) -> Option<String> {
    #[expect(
        clippy::disallowed_methods,
        reason = "the build script owns shelf's build-time process effects and runs git to read the commit"
    )]
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().to_owned())
}

/// Prints one instruction to cargo, which reads a build script's standard
/// output. Clippy's `print_stdout` lint does not apply to build scripts.
fn tell_cargo(instruction: &str) {
    println!("{instruction}");
}
