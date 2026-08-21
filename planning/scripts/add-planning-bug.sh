#!/usr/bin/env bash
# MODE: PROD
# add-planning-bug.sh — record one defect in a plan's planning-bugs.json.
#
# planning-bugs.json had readers in five places and nothing that wrote it, so an
# agent asking for it got exit 66 on every plan that ever existed. This is the
# writer, in the same shape as add-fix-claim.sh, which fixes.md needed for the
# same reason.
#
# Usage:
#   add-planning-bug.sh [--plan-dir] <plan-directory> --id <PB-NN> --title <text> \
#       --reproduce <text> --observed <text> --expected <text> \
#       [--severity blocking|major|minor|cosmetic] [--priority urgent|high|normal|low|someday] \
#       [--status reported|confirmed] [--found-by <text>]
#   add-planning-bug.sh --help
#
# The file follows the bug-report skill's schema, so its jq recipes read a plan's
# register unchanged. Defects about the work the plan describes belong here; a
# defect in the planning skill itself belongs in the repository's own BUGS.json.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> --id <PB-NN> --title <text> \\
           --reproduce <text> --observed <text> --expected <text> \\
           [--severity blocking|major|minor|cosmetic] \\
           [--priority urgent|high|normal|low|someday] \\
           [--status reported|confirmed] [--found-by <text>]
       ${0##*/} --help

  --id           plan-local id, PB-01 upward
  --reproduce    the command or steps, runnable rather than described
  --observed     what happened, quoted from the output
  --expected     what should have happened
  --severity     how bad it is when it happens; defaults to major
  --priority     when it gets fixed, a separate judgement; defaults to normal
  --status       reported (not yet reproduced) or confirmed; defaults to reported
  --found-by     who or what found it

Appends to <plan-directory>/planning-bugs.json, creating it on first use. Read it
back with plan-content.sh get <plan-directory> planning-bugs.
USAGE
    exit "$rc"
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"

bug_id=""
title=""
reproduce=""
observed=""
expected=""
severity="major"
priority="normal"
status="reported"
found_by=""
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --plan-dir) [ "$#" -ge 2 ] || usage; positional+=("$2"); shift 2 ;;
        --id) [ "$#" -ge 2 ] || usage; bug_id="$2"; shift 2 ;;
        --title) [ "$#" -ge 2 ] || usage; title="$2"; shift 2 ;;
        --reproduce) [ "$#" -ge 2 ] || usage; reproduce="$2"; shift 2 ;;
        --observed) [ "$#" -ge 2 ] || usage; observed="$2"; shift 2 ;;
        --expected) [ "$#" -ge 2 ] || usage; expected="$2"; shift 2 ;;
        --severity) [ "$#" -ge 2 ] || usage; severity="$2"; shift 2 ;;
        --priority) [ "$#" -ge 2 ] || usage; priority="$2"; shift 2 ;;
        --status) [ "$#" -ge 2 ] || usage; status="$2"; shift 2 ;;
        --found-by) [ "$#" -ge 2 ] || usage; found_by="$2"; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) positional+=("$1"); shift ;;
    esac
done
while [ "$#" -gt 0 ]; do positional+=("$1"); shift; done
set -- ${positional[@]+"${positional[@]}"}
[ "$#" -eq 1 ] || usage
plan_dir="$1"

# ---- validate before touching the file -------------------------------------
plan_require_directory "$plan_dir"
command -v jq >/dev/null 2>&1 \
    || plan_die "jq is required to write planning-bugs.json; the planning skill declares it in requires.tsv" 69
[ -n "$bug_id" ] || plan_die "--id is required (a plan-local id such as PB-01)"
[[ "$bug_id" =~ ^PB-[0-9]+$ ]] || plan_die "Bug id must match PB-NN: $bug_id"
for field in title reproduce observed expected; do
    eval "value=\${$field}"
    [ -n "$value" ] || plan_die "--$field is required; an entry without it cannot be acted on"
done
case "$severity" in
    blocking|major|minor|cosmetic) ;;
    *) plan_die "Severity must be blocking, major, minor or cosmetic: $severity" ;;
esac
case "$priority" in
    urgent|high|normal|low|someday) ;;
    *) plan_die "Priority must be urgent, high, normal, low or someday: $priority" ;;
esac
# Only the two open states: a writer that could record `fixed` would let an entry
# arrive already closed, with nothing that ever reproduced it.
case "$status" in
    reported|confirmed) ;;
    *) plan_die "Status must be reported or confirmed when recording a defect; close it later by editing the register with the bug-report skill: $status" ;;
esac

register="$plan_dir/planning-bugs.json"
if [ -f "$register" ]; then
    jq -e . "$register" >/dev/null 2>&1 \
        || plan_die "$register is not valid JSON; repair it before appending" 65
    if jq -e --arg id "$bug_id" 'any(.bugs[]?; .id == $id)' "$register" >/dev/null 2>&1; then
        plan_die "$bug_id is already recorded in planning-bugs.json; use a new id, or edit that entry" 73
    fi
fi

plan_git_snapshot "$plan_dir"

now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
# The current register, or an empty object on first use. Not /dev/null: jq given
# no input never runs the filter and writes nothing at all, which produced a
# zero-byte register and still reported success.
if [ -f "$register" ]; then
    current="$(cat "$register")"
else
    current='{}'
fi
# --arg everywhere: a title or a reproduction holding a quote, a backslash or a
# brace is ordinary here, and jq is what makes it survive into the file intact.
jq --arg id "$bug_id" --arg title "$title" --arg status "$status" \
   --arg severity "$severity" --arg priority "$priority" \
   --arg reproduce "$reproduce" --arg observed "$observed" --arg expected "$expected" \
   --arg found_by "$found_by" --arg now "$now" '
    (if type == "object" then . else {} end)
    | .skill = "bug-report"
    | .comment = (.comment // "Defects found while carrying out this plan.")
    | .bugs = ((.bugs // []) + [{
        id: $id, title: $title, status: $status,
        severity: $severity, priority: $priority, parent: null,
        reproduce: $reproduce, observed: $observed, expected: $expected,
        mechanism: null, surfaces: [], fix: null, verification: null,
        found_by: (if $found_by == "" then null else $found_by end),
        notes: null, created_at: $now, updated_at: $now
      }])' <<JSON | plan_atomic_write "$register"
$current
JSON

# A writer that reports success having written nothing is worse than one that
# fails: the caller believes the defect is recorded. Read the result back.
jq -e --arg id "$bug_id" 'any(.bugs[]?; .id == $id)' "$register" >/dev/null 2>&1 \
    || plan_die "wrote $register but $bug_id is not in it; the register may be damaged" 70

printf 'Recorded %s in %s\n' "$bug_id" "${register#"$plan_dir"/}"
