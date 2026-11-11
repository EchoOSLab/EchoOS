#!/usr/bin/env bash
# Q3-06：检查所有 commit subject 为英文 Conventional Commits。
set -uo pipefail
cd "$(dirname "$0")/../.."

pattern='^(feat|fix|docs|chore|test|perf|refactor|ci|build|revert)(\([a-z0-9._-]+\))?: .+$'
bad=0
while IFS= read -r subject; do
  if [[ ! "$subject" =~ $pattern ]]; then
    echo "BAD COMMIT: $subject"
    bad=$((bad + 1))
  fi
done < <(git log --format=%s)

if [ "$bad" -ne 0 ]; then
  echo "FAIL  commit lint: $bad bad subject(s)"
  exit 1
fi
echo "PASS  commit lint: all subjects are Conventional Commits"
