#!/usr/bin/env bash
# MODE: DEV
# test-session-start-hook -- decision-reminder-plugin's SessionStart hook
# names the open-question count and any urgent question's id and title,
# separately names every decided (answered, not yet implemented) question
# regardless of priority, and stays completely silent (no stdout, exit 0)
# when nothing is open or decided. Delete the --status open filter and the
# "silent when all closed" case fails; delete the urgent-line extraction and
# the urgent id/title assertion fails; delete the decided section and the
# decided-only case fails.
#
# Needs the real compiled `decisions` binary (the hook shells out to it, via
# hooks/lib.sh's decision_reminder_hook_decisions_bin), built on demand the
# same way test-register-schemas.sh builds the installer on demand.
#
# Usage:
#   test-session-start-hook.sh

set -uo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
plugin_dir="$(cd "$tests_dir/.." && pwd)"
repo_root="$(cd "$plugin_dir/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$repo_root/planning/tests/lib-test.sh"
t_begin

hook="$plugin_dir/hooks/session-start.sh"

decisions_bin="$repo_root/target/debug/decisions"
if [ ! -x "$decisions_bin" ] && command -v cargo >/dev/null 2>&1; then
    ( cd "$repo_root" && cargo build -p decisions ) >/dev/null 2>&1
fi
[ -x "$decisions_bin" ] || {
    printf 'decisions binary not built and cargo unavailable; run cargo build -p decisions\n' >&2
    exit 1
}

work="$(mktemp -d "${TMPDIR:-/tmp}/decision-reminder-hook.XXXXXX")"
trap 'rm -rf "$work"' EXIT

run_hook() { # <DECISIONS.json path> -> stdout of the hook
    env -u AI_SKILLS_BIN_ROOT AI_SKILLS_BIN_ROOT="$repo_root/target/debug" \
        DECISIONS_JSON="$1" "$hook"
}

register() { # <name> <json body>
    printf '%s' "$2" >"$work/$1"
    printf '%s\n' "$work/$1"
}

# ---- some open (one urgent), one decided, one implemented, one closed ------
mixed="$(register mixed.json '{
  "skill": "decisions",
  "skill_version": "2.0.0-alpha.5",
  "comment": "t",
  "questions": [
    {"id": "Q1", "title": "Cache the config, or re-read it every call?", "status": "open", "priority": "urgent", "branch": "main", "options": [{"letter": "a", "label": "cache"}, {"letter": "b", "label": "re-read"}], "context": "", "chosen": null, "resolution": null, "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
    {"id": "Q2", "title": "Which logger to use?", "status": "open", "priority": "normal", "branch": "main", "options": [{"letter": "a", "label": "log1"}, {"letter": "b", "label": "log2"}], "context": "", "chosen": null, "resolution": null, "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
    {"id": "Q3", "title": "Decided but not yet implemented", "status": "decided", "priority": "normal", "branch": "main", "options": [{"letter": "a", "label": "x"}], "context": "", "chosen": "a", "resolution": null, "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
    {"id": "Q4", "title": "Already implemented one", "status": "implemented", "priority": "urgent", "branch": "main", "options": [{"letter": "a", "label": "x"}], "context": "", "chosen": "a", "resolution": "done", "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
    {"id": "Q5", "title": "Already resolved one", "status": "closed", "priority": "urgent", "branch": "main", "options": [{"letter": "a", "label": "x"}], "context": "", "chosen": "a", "resolution": "done", "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"}
  ]
}')"

out="$(run_hook "$mixed")"
t_assert_contains 'the reminder is a SessionStart additionalContext' '"hookEventName":"SessionStart"' "$out"
t_assert_contains 'the open count is the open ones only (2), not every question' '2 open question' "$out"
t_assert_contains 'the urgent open question is named by id' 'Q1' "$out"
t_assert_contains 'the urgent open question is named by title' 'Cache the config' "$out"
case "$out" in
    *'Urgent: Q2'*) t_fail 'a non-urgent open question must not be individually named as urgent' ;;
    *) : ;;
esac
case "$out" in
    *'Q5'*) t_fail 'a closed question must never appear, urgent or not' ;;
    *) : ;;
esac
case "$out" in
    *'Q4'*) t_fail 'an already-implemented question must never appear' ;;
    *) : ;;
esac
t_assert_contains 'the reminder says it is non-blocking' 'Non-blocking' "$out"
t_assert_contains 'the decided count is named' '1 decided question' "$out"
t_assert_contains 'the decided question is named by id regardless of priority' 'Q3' "$out"
t_assert_contains 'the decided question is named by title' 'Decided but not yet implemented' "$out"
t_assert_contains 'the decided section tells the agent to act' 'implement' "$out"
case "$out" in
    *$'\n'*) t_fail 'the output spans raw lines, so a newline was not escaped' ;;
    *) : ;;
esac

# ---- nothing open, one decided: still non-silent, decided-only wording -----
decided_only="$(register decided-only.json '{
  "skill": "decisions",
  "skill_version": "2.0.0-alpha.5",
  "comment": "t",
  "questions": [
    {"id": "Q1", "title": "Pick the backend", "status": "decided", "priority": "low", "branch": "main", "options": [{"letter": "a", "label": "x"}], "context": "", "chosen": "a", "resolution": null, "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"}
  ]
}')"
decided_out="$(run_hook "$decided_only")"
case "$decided_out" in
    *'open question'*) t_fail 'no open questions exist; the open paragraph must not appear' ;;
    *) : ;;
esac
t_assert_contains 'a decided-only register still produces a reminder' 'Q1' "$decided_out"
t_assert_contains 'the decided-only reminder names the title' 'Pick the backend' "$decided_out"

# ---- everything implemented/closed: silent, no additionalContext at all ----
all_closed="$(register all-closed.json '{
  "skill": "decisions",
  "skill_version": "2.0.0-alpha.5",
  "comment": "t",
  "questions": [
    {"id": "Q1", "title": "Resolved", "status": "closed", "priority": "urgent", "branch": "main", "options": [{"letter": "a", "label": "x"}], "context": "", "chosen": "a", "resolution": "done", "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"},
    {"id": "Q2", "title": "Done", "status": "implemented", "priority": "urgent", "branch": "main", "options": [{"letter": "a", "label": "x"}], "context": "", "chosen": "a", "resolution": "done", "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"}
  ]
}')"
closed_out="$(run_hook "$all_closed")"
t_assert_eq 'nothing open or decided means completely silent stdout' "$closed_out" ''

# ---- no register at all: silent, same as nothing open ----------------------
missing_out="$(run_hook "$work/no-such-file.json")"
t_assert_eq 'a missing register means completely silent stdout' "$missing_out" ''

t_end 'test-session-start-hook'
