# Brief: a timed-out run is reaped even when the kill fails

You are a Claude Code cloud session. This run is also a trial of a setup
script for this repository.

## Setup

- Install the toolchain pinned in `rust-toolchain.toml`.
- Save the script below as `/tmp/setup.sh` and run it unchanged with
  `bash /tmp/setup.sh` from the parent directory of the repository.
- If a line fails, record the line and the error, fix it with the smallest
  change, and put the corrected whole script in the environment report.

```bash
#!/usr/bin/env bash
set -euo pipefail
if [ -d crumbkit ]; then
  cd crumbkit
  rustup toolchain install 1.99.1 --profile minimal -c clippy -c rustfmt
  rustup toolchain install 1.89.0 --profile minimal
  cargo +1.99.1 install --locked cargo-nextest cargo-deny cargo-shear
  pip install zizmor
fi
```

## The change

- File: `crates/crumbkit-test/src/runs.rs`, where a run that passes its
  timeout is killed with `child.kill()?; child.wait()?;`.
- Defect: when the kill returns an error, `?` leaves the function before
  `wait`, so the child is never reaped. The reader and writer threads the
  module spawns are also never joined.
- Fix: call `wait` after `kill` whatever the kill returned, then return the
  kill's error if there was one. Join the reader and writer threads where
  that cannot block a check on a run that never reads its input or never
  closes its output; where a join could block, leave it and say why in a
  comment at that spot.
- Test first: wrap the child in a type whose `kill` fails, and assert that
  `wait` still ran. The test fails before the fix.
- Keep the change small. Change no public API.

## How to work

- Read `AGENTS.md` first and follow it, including its check list.
- Work on a new branch named `runs-wait-after-kill`, cut from `main`. Leave
  `BRIEF.md` out of it.
- Run every check in `AGENTS.md` before you push. Run zizmor as
  `zizmor --offline .`.
- These four tests fail only in this container and are left to CI. Any
  other failure is yours to fix.
  - `a_write_that_fails_gives_the_operating_system_s_error_as_the_reason`
    (the container runs tests as root)
  - the three `a_run_leaves_no_process_of_the_group_*` tests in
    `crumbkit-process` (the container's first process does not reap
    orphans)
- Add a `CHANGELOG.md` entry if `AGENTS.md` asks for one.

## The pull request

- Open one pull request against `main`, marked ready, and wait for its CI
  to pass. Fix what CI finds.
- The commit message and the title describe the behaviour.
- Add no attribution lines and name no AI model anywhere.
- Name no company, product or repository outside the crumbkit organisation.
- Never merge the pull request.
- End the body with a section "Environment report": whether the setup
  script ran unchanged, the corrected script if it did not, each further
  tool you had to install, each command that failed and why, and anything
  the environment blocked.

## When you are stuck

- Ask nobody. Stop, and write what blocked you in the pull request body, or
  in your final message when no pull request exists.
