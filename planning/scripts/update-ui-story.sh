#!/usr/bin/env bash
# MODE: PROD
# update-ui-story.sh — correct the narrative columns of one UI story row.
#
# add-ui-story.sh appends and add-ui-story-links.sh rewrites the related work
# units, but nothing could correct a story's own text, so a story that turned out
# to contradict the plan had to be left wrong or edited by hand. Editing plan
# files by hand is what the helpers exist to prevent.
#
# The interaction rule is re-checked against the resulting row, not the arguments:
# correcting only the actions column could otherwise leave a row whose remaining
# text names no interaction at all.
#
# Usage:
#   update-ui-story.sh [--plan-dir] <plan-directory> <US-NN>
#       [--persona <text>] [--actions <text>] [--interaction <text>]
#       [--expected <text>]
#   update-ui-story.sh --help
#
# Exit codes: 64 bad invocation, 66 the plan file or the story is missing.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <US-NN>
           [--persona <text>] [--actions <text>] [--interaction <text>]
           [--expected <text>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

plan_dir=''
story_id=''
persona=''
actions=''
interaction=''
expected=''
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --persona) [ "$#" -ge 2 ] || usage; persona="$2"; shift 2 ;;
        --actions) [ "$#" -ge 2 ] || usage; actions="$2"; shift 2 ;;
        --interaction) [ "$#" -ge 2 ] || usage; interaction="$2"; shift 2 ;;
        --expected) [ "$#" -ge 2 ] || usage; expected="$2"; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *)
            if [ -z "$plan_dir" ]; then plan_dir="$1"
            elif [ -z "$story_id" ]; then story_id="$1"
            else usage
            fi
            shift
            ;;
    esac
done
[ -n "$plan_dir" ] && [ -n "$story_id" ] || usage
[ -n "$persona$actions$interaction$expected" ] || usage

plan_require_directory "$plan_dir"
[[ "$story_id" =~ ^US-[0-9][0-9]+$ ]] || plan_die "Story ID must use US-01"
for value_name in persona actions interaction expected; do
    [ -z "${!value_name}" ] || plan_require_safe_value "$value_name" "${!value_name}"
done

stories="$plan_dir/ui-user-stories.md"
[ -f "$stories" ] || plan_die "UI story artifact not found; run create-ui-validation.sh first" 66

row="$(awk -v wanted="$story_id" '
    BEGIN { FS = "|" }
    function t(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v }
    /^\|/ && t($2) == wanted { print; exit }
' "$stories")"
[ -n "$row" ] || plan_die "Story ID not found: $story_id" 66

# The row's current values, so an unspecified column keeps what it had.
current() { # <field-number>
    printf '%s' "$row" | awk -F'|' -v n="$1" '{
        v = $n
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
        print v
    }'
}
[ -n "$persona" ] || persona="$(current 3)"
[ -n "$actions" ] || actions="$(current 4)"
[ -n "$interaction" ] || interaction="$(current 5)"
[ -n "$expected" ] || expected="$(current 6)"

[[ "$actions $interaction" =~ [Cc]lick|[Tt]ap|[Tt]ype|[Kk]eyboard|[Pp]ress|[Ss]wipe|[Pp]inch|[Dd]rag|[Ss]elect ]] \
    || plan_die "A UI story must name a direct user interaction (accepted verbs: click, tap, type, keyboard, press, swipe, pinch, drag, select)"

plan_git_snapshot "$plan_dir"
awk -v wanted="$story_id" -v p="$persona" -v a="$actions" -v i="$interaction" -v e="$expected" '
    BEGIN { FS = "|"; OFS = "|" }
    function t(v) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v }
    /^\|/ && t($2) == wanted {
        $3 = " " p " "
        $4 = " " a " "
        $5 = " " i " "
        $6 = " " e " "
        touched = 1
    }
    { print }
    END { if (!touched) exit 2 }
' "$stories" | plan_atomic_write "$stories" || plan_die "UI story row not found"
printf 'Updated %s\n' "$story_id"
