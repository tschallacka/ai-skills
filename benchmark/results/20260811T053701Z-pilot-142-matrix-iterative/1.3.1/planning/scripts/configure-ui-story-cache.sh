#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 7 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <US-NN> <starting-state> <direct-ui-input> <target-or-value> <readiness-signal> <maximum-wait>" >&2
    exit 64
fi

plan_dir="$1"; story_id="$2"; starting_state="$3"; direct_input="$4"; target="$5"; readiness="$6"; maximum_wait="$7"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$plan_dir"
[[ "$story_id" =~ ^US-[0-9][0-9]+$ ]] || plan_die "Story ID must use US-01"
for value_name in starting_state direct_input target readiness maximum_wait; do
    plan_require_safe_value "$value_name" "${!value_name}"
done
[[ "$direct_input" =~ [Cc]lick|[Tt]ap|[Tt]ype|[Kk]eyboard|[Pp]ress|[Ss]wipe|[Pp]inch|[Dd]rag|[Ss]elect ]] || plan_die "Direct UI input must name a real user interaction"
cache_file="$plan_dir/ui-story-runs/$story_id.md"
[ -f "$cache_file" ] || plan_die "Browser run cache not found: $cache_file"

temporary_file="${cache_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
{
    printf '# Browser run cache: %s\n\n' "$story_id"
    printf '## Starting state\n\n- %s\n\n' "$starting_state"
    printf '## Buffered interaction sequence\n\n'
    printf '| Order | Direct UI input | Target / value | Expected readiness signal |\n|---|---|---|---|\n'
    printf '| 1 | %s | %s | %s |\n\n' "$direct_input" "$target" "$readiness"
    printf '## Waits and readiness\n\n'
    printf '| After order | Wait or condition | Maximum wait | Observed result |\n|---|---|---|---|\n'
    printf '| 1 | %s | %s | Not run yet |\n\n' "$readiness" "$maximum_wait"
    printf '## Run result\n\n'
    printf '%s\n' '- Status: `💤 untested`'
    printf '%s\n' '- Evidence: Pending browser run.'
    printf '%s\n' '- Cache validity: Created for the current planned interaction sequence.'
} > "$temporary_file"
mv "$temporary_file" "$cache_file"
trap - EXIT
echo "Configured $cache_file"
