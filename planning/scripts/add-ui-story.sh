#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 7 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <US-NN> <persona-or-precondition> <browser-actions> <interaction-evidence> <expected-result> <WNN[,WNN...]>" >&2
    exit 64
fi

plan_dir="$1"; story_id="$2"; persona="$3"; actions="$4"; interaction="$5"; expected="$6"; work_units="$7"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$plan_dir"
[[ "$story_id" =~ ^US-[0-9][0-9]+$ ]] || plan_die "Story ID must use US-01"
[[ "$work_units" =~ ^W[0-9][0-9]+(,[[:space:]]*W[0-9][0-9]+)*$ ]] || plan_die "Work units must be comma-separated IDs such as W01,W02"
for value_name in persona actions interaction expected; do
    plan_require_safe_value "$value_name" "${!value_name}"
done
[[ "$actions $interaction" =~ [Cc]lick|[Tt]ap|[Tt]ype|[Kk]eyboard|[Pp]ress|[Ss]wipe|[Pp]inch|[Dd]rag|[Ss]elect ]] \
    || plan_die "A UI story must name a direct user interaction (accepted verbs: click, tap, type, keyboard, press, swipe, pinch, drag, select)"
stories="$plan_dir/ui-user-stories.md"
[ -f "$stories" ] || plan_die "UI story artifact not found; run create-ui-validation.sh first"
if awk -F'|' -v wanted="$story_id" 'function t(v){gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v} /^\|/ && t($2)==wanted {found=1} END {exit !found}' "$stories"; then
    plan_die "Story ID already exists: $story_id"
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
trap - EXIT
"$script_dir/create-ui-story-run-cache.sh" "$plan_dir" "$story_id" >/dev/null
echo "Added $story_id"
