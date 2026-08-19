#!/usr/bin/env bash
# create-ui-story-run-cache.sh — create the empty browser run cache for one UI
# story: its starting state, one buffered interaction, its readiness wait, and an
# untested run result.
#
# add-ui-story.sh calls this, so the cache exists from the moment the story does.
# Every field is the literal "not yet configured" until
# configure-ui-story-cache.sh writes the real sequence; run that next.
#
# Usage:
#   create-ui-story-run-cache.sh [--plan-dir] <plan-directory> <US-NN>
#   create-ui-story-run-cache.sh --help

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
Usage: ${0##*/} [--plan-dir] <plan-directory> <US-NN>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -eq 2 ] || usage

plan_dir="$1"
story_id="$2"
if [ ! -d "$plan_dir" ]; then
    echo "Plan directory not found: $plan_dir" >&2
    exit 66
fi
if [[ ! "$story_id" =~ ^US-[0-9][0-9]+$ ]]; then
    echo "Invalid story ID: $story_id (use US-01)" >&2
    exit 64
fi

cache_dir="$plan_dir/ui-story-runs"
cache_file="$cache_dir/$story_id.md"
if [ -e "$cache_file" ]; then
    echo "Browser run cache already exists: $cache_file" >&2
    exit 73
fi

mkdir -p "$cache_dir"
temporary_file="${cache_file}.tmp.$$"
trap 'rm -f "$temporary_file"' EXIT
{
    printf '# Browser run cache: %s\n\n' "$story_id"
    printf '## Starting state\n\n'
    printf '%s\n\n' '- URL, persona, viewport/device, and visible initial condition: not yet configured'
    printf '## Buffered interaction sequence\n\n'
    printf '| Order | Direct UI input | Target / value | Expected readiness signal |\n'
    printf '|---|---|---|---|\n'
    printf '| 1 | one direct user interaction (click, tap, type, keyboard, press, swipe, pinch, drag, select) | the target element and value | the observable readiness signal |\n\n'
    printf '## Waits and readiness\n\n'
    printf '| After order | Wait or condition | Maximum wait | Observed result |\n'
    printf '|---|---|---|---|\n'
    printf '| 1 | the readiness condition | the maximum wait | the actual elapsed wait |\n\n'
    printf '## Run result\n\n'
    printf '%s\n' '- Status: `💤 untested`'
    printf '%s\n' '- Evidence: not yet configured (run configure-ui-story-cache.sh to set the sequence, then execute)'
    printf '%s\n' '- Cache validity: not yet configured'
} > "$temporary_file"
mv "$temporary_file" "$cache_file"

printf 'Created %s\n' "$cache_file"
