#!/usr/bin/env bash
# MODE: PROD
# configure-ui-story-cache.sh — fill a UI story's browser run cache with the one
# buffered interaction the story will actually perform, plus its readiness wait.
#
# This rewrites the whole cache file from the arguments (it is a generated
# document, so hand edits are not preserved) and resets the run result to
# untested: a configured sequence has by definition not been run yet.
#
# Usage:
#   configure-ui-story-cache.sh [--plan-dir] <plan-directory> --id <US-NN> \
#       --starting-state <text> --input <direct UI input> --target <text> \
#       --readiness <text> --max-wait <text>
#   configure-ui-story-cache.sh [--plan-dir] <plan-directory> <US-NN> <starting-state> \
#       <direct-ui-input> <target-or-value> <readiness-signal> <maximum-wait>
#   configure-ui-story-cache.sh --help
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
Usage: ${0##*/} [--plan-dir] <plan-directory> --id <US-NN> --starting-state <text>
           --input <direct UI input> --target <text> --readiness <text>
           --max-wait <text>
       ${0##*/} [--plan-dir] <plan-directory> <US-NN> <starting-state> <direct-ui-input> <target-or-value> <readiness-signal> <maximum-wait>
       ${0##*/} --help

The positional form is deprecated; it is kept working for existing callers.
USAGE
    exit "$rc"
}

story_id=""
starting_state=""
direct_input=""
target=""
readiness=""
maximum_wait=""
flags_used=false
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --id) [ "$#" -ge 2 ] || usage; story_id="$2"; flags_used=true; shift 2 ;;
        --starting-state) [ "$#" -ge 2 ] || usage; starting_state="$2"; flags_used=true; shift 2 ;;
        --input) [ "$#" -ge 2 ] || usage; direct_input="$2"; flags_used=true; shift 2 ;;
        --target) [ "$#" -ge 2 ] || usage; target="$2"; flags_used=true; shift 2 ;;
        --readiness) [ "$#" -ge 2 ] || usage; readiness="$2"; flags_used=true; shift 2 ;;
        --max-wait) [ "$#" -ge 2 ] || usage; maximum_wait="$2"; flags_used=true; shift 2 ;;
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
    for required in story_id starting_state direct_input target readiness maximum_wait; do
        [ -n "${!required}" ] || usage
    done
else
    # Deprecated positional form.
    [ "$#" -eq 7 ] || usage
    plan_dir="$1"; story_id="$2"; starting_state="$3"; direct_input="$4"
    target="$5"; readiness="$6"; maximum_wait="$7"
fi


plan_require_directory "$plan_dir"
[[ "$story_id" =~ ^US-[0-9][0-9]+$ ]] || plan_die "Story ID must use US-01"
for value_name in starting_state direct_input target readiness maximum_wait; do
    plan_require_safe_value "$value_name" "${!value_name}"
done
[[ "$direct_input" =~ [Cc]lick|[Tt]ap|[Tt]ype|[Kk]eyboard|[Pp]ress|[Ss]wipe|[Pp]inch|[Dd]rag|[Ss]elect ]] || plan_die "Direct UI input must name a real user interaction"
cache_file="$plan_dir/ui-story-runs/$story_id.md"
if [ ! -f "$cache_file" ]; then
    printf '%s: %s\n' "${0##*/}" "Browser run cache not found: $cache_file" >&2
    exit 66
fi

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
printf 'Configured %s\n' "$cache_file"
