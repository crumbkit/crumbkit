#!/bin/sh
# Checks each commit after the first argument up to the second argument
# against the case-insensitive extended regular expression in COMMIT_PATTERN
# (ADR-0011). It checks the author name, author email, committer name,
# committer email and message of each commit with `grep -Eiq`.
#
# For each match it prints the commit's SHA and the name of the field that
# matched, and never the field's value, and it exits with status 1 after
# checking every commit. It exits with status 2 when grep rejects the
# pattern. When COMMIT_PATTERN is empty, as in a pull request from a fork, it
# prints a notice that it skipped the check and exits with status 0.
set -eu

base=$1
head=$2
pattern=${COMMIT_PATTERN:-}

if [ -z "$pattern" ]; then
  echo "::notice::COMMIT_PATTERN is empty, so the commit check was skipped."
  exit 0
fi

matched=0
for sha in $(git rev-list "$base..$head"); do
  # Each entry pairs a git log placeholder with the field name it prints.
  for field in "an:author name" "ae:author email" "cn:committer name" \
    "ce:committer email" "B:message"; do
    status=0
    git log -1 --format="%${field%%:*}" "$sha" |
      grep -Eiq -e "$pattern" 2>/dev/null || status=$?
    case $status in
      0)
        echo "::error::Commit $sha: the ${field#*:} matches COMMIT_PATTERN."
        matched=1
        ;;
      1) ;;
      *)
        echo "::error::COMMIT_PATTERN is not a valid extended regular expression."
        exit 2
        ;;
    esac
  done
done
exit "$matched"
