#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <US-NN>" >&2
    exit 64
fi

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
    printf '%s\n\n' '- URL, persona, viewport/device, and visible initial condition'
    printf '## Buffered interaction sequence\n\n'
    printf '| Order | Direct UI input | Target / value | Expected readiness signal |\n'
    printf '|---|---|---|---|\n'
    printf '| 1 | <mouse click, keyboard input, or mobile gesture> | <target / value> | <observable state> |\n\n'
    printf '## Waits and readiness\n\n'
    printf '| After order | Wait or condition | Maximum wait | Observed result |\n'
    printf '|---|---|---|---|\n'
    printf '| 1 | <readiness condition> | <maximum wait> | <actual elapsed wait> |\n\n'
    printf '## Run result\n\n'
    printf '%s\n' '- Status: `💤 untested`'
    printf '%s\n' '- Evidence: <screenshot, URL, and observed result after execution>'
    printf '%s\n' '- Cache validity: <current UI revision or why the sequence remains valid>'
} > "$temporary_file"
mv "$temporary_file" "$cache_file"
trap - EXIT

echo "Created $cache_file"
