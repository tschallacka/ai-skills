#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# add-ui-story.sh — add one UI user story row to a plan's ui-user-stories.md and
# create its browser run cache.
#
# The story must name a direct user interaction (click, tap, type, keyboard,
# press, swipe, pinch, drag, select): a story nobody can perform in a browser is
# not evidence. The row starts at "💤 untested" with no evidence; a run fills it.
#
# Usage:
#   add-ui-story.sh [--plan-dir] <plan-directory> --id <US-NN> --persona <text> \
#       --actions <text> --interaction <text> --expected <text> \
#       --work-units <WNN[,WNN...]>
#   add-ui-story.sh [--plan-dir] <plan-directory> <US-NN> <persona-or-precondition> \
#       <browser-actions> <interaction-evidence> <expected-result> <WNN[,WNN...]>
#   add-ui-story.sh --help
#
# The second form is the deprecated positional spelling, kept working for
# existing callers.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> --id <US-NN> --persona <text> --actions <text>
           --interaction <text> --expected <text> --work-units <WNN[,WNN...]>
       ${0##*/} [--plan-dir] <plan-directory> <US-NN> <persona-or-precondition> <browser-actions> <interaction-evidence> <expected-result> <WNN[,WNN...]>
       ${0##*/} --help

The positional form is deprecated; it is kept working for existing callers.
USAGE
    exit "$rc"
}

story_id=""
persona=""
actions=""
interaction=""
expected=""
work_units=""
flags_used=false
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --id) [ "$#" -ge 2 ] || usage; story_id="$2"; flags_used=true; shift 2 ;;
        --persona) [ "$#" -ge 2 ] || usage; persona="$2"; flags_used=true; shift 2 ;;
        --actions) [ "$#" -ge 2 ] || usage; actions="$2"; flags_used=true; shift 2 ;;
        --interaction) [ "$#" -ge 2 ] || usage; interaction="$2"; flags_used=true; shift 2 ;;
        --expected) [ "$#" -ge 2 ] || usage; expected="$2"; flags_used=true; shift 2 ;;
        --work-units) [ "$#" -ge 2 ] || usage; work_units="$2"; flags_used=true; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) positional+=("$1"); shift ;;
    esac
done
while [ "$#" -gt 0 ]; do positional+=("$1"); shift; done

set -- ${positional[@]+"${positional[@]}"}
if [ "$flags_used" = true ]; then
    [ "$#" -eq 1 ] || usage
    plan_dir="$1"
    for required in story_id persona actions interaction expected work_units; do
        [ -n "${!required}" ] || usage
    done
else
    # Deprecated positional form.
    [ "$#" -eq 7 ] || usage
    plan_dir="$1"; story_id="$2"; persona="$3"; actions="$4"; interaction="$5"
    expected="$6"; work_units="$7"
fi


plan_require_directory "$plan_dir"
[[ "$story_id" =~ ^US-[0-9][0-9]+$ ]] || plan_die "Story ID must use US-01"
[[ "$work_units" =~ ^W[0-9][0-9]+(,[[:space:]]*W[0-9][0-9]+)*$ ]] || plan_die "Work units must be comma-separated IDs such as W01,W02"
for value_name in persona actions interaction expected; do
    plan_require_safe_value "$value_name" "${!value_name}"
done
[[ "$actions $interaction" =~ [Cc]lick|[Tt]ap|[Tt]ype|[Kk]eyboard|[Pp]ress|[Ss]wipe|[Pp]inch|[Dd]rag|[Ss]elect ]] \
    || plan_die "A UI story must name a direct user interaction (accepted verbs: click, tap, type, keyboard, press, swipe, pinch, drag, select)"
stories="$plan_dir/ui-user-stories.md"
if [ ! -f "$stories" ]; then
    printf '%s: %s\n' "${0##*/}" 'UI story artifact not found; run create-ui-validation.sh first' >&2
    exit 66
fi
if awk -F'|' -v wanted="$story_id" 'function t(v){gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v} /^\|/ && t($2)==wanted {found=1} END {exit !found}' "$stories"; then
    printf '%s: %s\n' "${0##*/}" "Story ID already exists: $story_id" >&2
    exit 73
fi

cache_path="ui-story-runs/$story_id.md"
temporary_file="${stories}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
awk -v row="| $story_id | $persona | $actions | $interaction | $expected | 💤 untested | — | $work_units | \`$cache_path\` |" '
    /^\|---/ && !inserted { print; print row; inserted = 1; next }
    { print }
    END { if (!inserted) exit 2 }
' "$stories" > "$temporary_file" || plan_die "UI story table header not found"
mv "$temporary_file" "$stories"
"$script_dir/create-ui-story-run-cache.sh" "$plan_dir" "$story_id" >/dev/null
printf 'Added %s\n' "$story_id"
