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
# tests"; separate paragraphs with "\n" escapes or real newlines
# (multi-paragraph companions are supported and are what reviewers actually
# proofread; every paragraph gets its own § 2.N label).

overwrite=false
filtered_args=()
for arg in "$@"; do
    case "$arg" in
        -h|--help)
            printf 'Usage: %s <goal-directory> <step-name> <verification-instructions> [--overwrite]\n' "$(basename "$0")" >&2
            exit 0
            ;;
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

# Multi-paragraph instructions: every paragraph becomes its own numbered §2.x
# block. Paragraphs may be separated by literal "\n" escapes (documented
# convention) or by real newlines/blank lines. Normalize escapes to real
# newlines, then split on any newline run, so both spellings produce one label
# per paragraph — an unlabeled paragraph would silently receive edits aimed at
# the labeled one. split() with a regex separator (/\\n/ matches literal
# backslash-n, /\n+/ matches newline runs) keeps mawk and one-true-awk/BSD awk
# consistent, unlike a multi-char RS which mawk treats as a regex but others
# match literally.
body_file="$(mktemp "${TMPDIR:-/tmp}/plan-testing.XXXXXX")"
trap 'rm -f "$body_file"' EXIT
printf '%s' "$instructions" | awk '
    { text = text $0 "\n" }
    END {
        gsub(/\\n/, "\n", text)
        n = split(text, paras, /\n+/)
        for (i = 1; i <= n; i++) {
            para = paras[i]
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", para)
            if (para == "") continue
            if (count++) printf "\n\n"
            printf "§ 2.%d\n%s", count, para
        }
        if (count == 0) exit 1
    }
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