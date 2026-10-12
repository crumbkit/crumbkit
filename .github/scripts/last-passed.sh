#!/bin/sh
# Prints the commit of the newest completed run of scheduled.yml on main in
# which the job named by the first argument passed, or nothing when none of
# the ten newest completed runs has one. A scheduled job compares the commit
# with its own and skips its checks when the two are equal (ADR-0011).
#
# It needs GH_TOKEN and GITHUB_REPOSITORY, which GitHub Actions sets, and the
# gh CLI. It reads at most ten runs, so after ten runs without a pass of the
# job it prints nothing and the job runs its checks.
set -eu

job=$1
for run in $(gh api "repos/$GITHUB_REPOSITORY/actions/workflows/scheduled.yml/runs?branch=main&status=completed&per_page=10" \
  --jq '.workflow_runs[] | "\(.id):\(.head_sha)"'); do
  conclusion=$(gh api "repos/$GITHUB_REPOSITORY/actions/runs/${run%%:*}/jobs" \
    --jq ".jobs[] | select(.name == \"$job\") | .conclusion")
  if [ "$conclusion" = success ]; then
    echo "${run#*:}"
    exit 0
  fi
done
