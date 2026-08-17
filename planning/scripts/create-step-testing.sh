#!/usr/bin/env bash
set -euo pipefail

# create-step-testing.sh — create or (with --overwrite) replace a step's
# testing companion. Input is validated BEFORE any filesystem change, so a
# rejected call never leaves the plan with the old companion already deleted.
#
# Usage:
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions>
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions> --overwrite
#
# The instructions are rendered as a numbered §2.x section under "## Automated
# tests"; use "\n" to separate paragraphs (multi-paragraph companions are
# supported and are what reviewers actually proofread).

overwrite=false
filtered_args=()
for arg in "$@"; do
    case "$arg" in
        --overwrite) overwrite=true ;;
        *) filtered_args+=("$arg") ;;
    esac
done
set -- "${filtered_args[@]}"

if [ "$#" -ne 3 ]; then
    printf 'Usage: %s <goal-directory> <step-name> <verification-instructions> [--overwrite]\n' "$(basename "$0")" >&2
    exit 64
fi

goal_dir="$1"
step_name="$2"
instructions="$3"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

# ---- validate everything BEFORE touching the filesystem ----
plan_require_directory "$goal_dir"
[[ "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]] || plan_die "Step name must use 01-step-kebab-case"
step_file="$goal_dir/steps/$step_name.md"
[ -f "$step_file" ] || plan_die "Implementation step not found: $step_file"
testing_file="$goal_dir/steps/${step_name}-testing.md"
if [ -e "$testing_file" ] && [ "$overwrite" = false ]; then
    plan_die "Testing companion already exists: $testing_file (pass --overwrite to replace it)"
fi
[ -n "${instructions//[[:space:]]/}" ] || plan_die "Verification instructions must not be empty"
[[ "$instructions" != *'|'* ]] || plan_die "Verification instructions must not contain a Markdown table separator (|)"
[[ "$instructions" != *'§'* ]] || plan_die "Verification instructions must not contain the reserved paragraph marker §"

# Multi-paragraph instructions: split on literal "\n" into numbered §2.x blocks.
body_file="$(mktemp "${TMPDIR:-/tmp}/plan-testing.XXXXXX")"
trap 'rm -f "$body_file"' EXIT
printf '%s' "$instructions" | awk '
    BEGIN { RS = "\\\\n"; ORS = "" }
    {
        text = $0
        sub(/^[[:space:]\n]+/, "", text)
        sub(/[[:space:]\n]+$/, "", text)
        if (text == "") next
        if (count++) printf "\n\n"
        printf "§ 2.%d\n%s", count, text
    }
    END { if (count == 0) exit 1 }
' > "$body_file" || plan_die "Verification instructions are empty after splitting"

temporary_file="${testing_file}.tmp.$$"
trap 'rm -f "$body_file" "$temporary_file"' EXIT
{
    printf '# Verification: %s\n\n' "$step_name"
    printf '## Automated tests\n\n'
    cat "$body_file"
} > "$temporary_file"
mv "$temporary_file" "$testing_file"
trap - EXIT
printf 'Created testing companion %s\n' "$testing_file"