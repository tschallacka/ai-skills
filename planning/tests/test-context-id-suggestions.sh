#!/usr/bin/env bash
# test-context-id-suggestions.sh — an unsupported entry id must say what to use
# instead. See CODE-CONTRACTS.md contract 6.
#
# A reviewer asked the reader for `progress:<goal>`. The real id is
# `goal-progress:<goal>`, but the refusal said only "unsupported entry id", and
# the reviewer concluded the per-goal progress trackers could not be served at
# all -- so they went unreviewed for a whole release. The documents were servable
# the entire time; only the message was a dead end.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/context-id-suggestions.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan" "$plan"
goal='01-plan-dir-synonym'
"$scripts_dir/plan-context.sh" init --plan-dir "$plan" >/dev/null 2>&1

read_id() { "$scripts_dir/plan-context.sh" read --plan-dir "$plan" --document "$1" 2>&1; }

# ---- the per-goal tracker is servable, which is the premise ------------------
rc=0
served="$(read_id "goal-progress:$goal")" || rc=$?
t_assert_eq 'the per-goal progress tracker is servable' "$rc" 0
case "$served" in
    *"Progress: $goal"*) ;;
    *) t_fail "goal-progress did not return the tracker: $served" ;;
esac

# ---- the near miss is named --------------------------------------------------
rc=0
refused="$(read_id "progress:$goal")" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'an unsupported id was accepted'
case "$refused" in
    *"goal-progress:$goal"*) ;;
    *) t_fail "the refusal did not name the id to use instead: $refused" ;;
esac

# ---- an id with no near miss still names the vocabulary ----------------------
rc=0
unknown="$(read_id 'nonsense')" || rc=$?
[ "$rc" -ne 0 ] || t_fail 'a nonsense id was accepted'
for expected_id in plan inventory 'goal-progress:<goal>' 'unit:WNN'; do
    case "$unknown" in
        *"$expected_id"*) ;;
        *) t_fail "the refusal for an unknown id did not name $expected_id: $unknown" ;;
    esac
done

t_end
