#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-progress-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

goal_dir="$temporary_root/01-demo"
mkdir -p "$goal_dir/steps"
printf '# First step\n' > "$goal_dir/steps/01-first.md"

"$script_dir/create-progress.sh" "$goal_dir" demo
"$script_dir/update-step.sh" "$goal_dir" 01-first completed

grep -Fqx '| demo | 01-first | <short description> | ✅ completed |' "$goal_dir/progress.md"
grep -Fqx '**Progress:** `100%  ####################  100%` ✅' "$goal_dir/progress.md"

printf 'Progress helper regression test passed.\n'
