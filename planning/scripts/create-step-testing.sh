#!/usr/bin/env bash
# create-step-testing.sh — create or (with --overwrite) replace a step's
# testing companion. Input is validated BEFORE any filesystem change, so a
# rejected call never leaves the plan with the old companion already deleted.
#
# Usage:
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions>
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions> --overwrite
#   create-step-testing.sh --help
#
# The instructions are rendered as a numbered §2.x section under "## Automated
# tests"; separate paragraphs with "\n" escapes or real newlines
# (multi-paragraph companions are supported and are what reviewers actually
# proofread; every paragraph gets its own § 2.N label).

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <goal-directory> <step-name> <verification-instructions> [--overwrite]
       ${0##*/} --help
USAGE
    exit "$rc"
}

# A flag loop, not a "$@" pre-scan into an array: -h belongs in band, and
# re-setting "$@" from a possibly-empty array aborts under set -u on bash 3.2.
overwrite=false
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --overwrite) overwrite=true; shift ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) positional+=("$1"); shift ;;
    esac
done
while [ "$#" -gt 0 ]; do positional+=("$1"); shift; done

set -- ${positional[@]+"${positional[@]}"}
[ "$#" -eq 3 ] || usage

goal_dir="$1"
step_name="$2"
instructions="$3"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

# ---- validate everything BEFORE touching the filesystem ----
plan_require_directory "$goal_dir"
[[ "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]] || plan_die "Step name must use 01-step-kebab-case"
step_file="$goal_dir/steps/$step_name.md"
if [ ! -f "$step_file" ]; then
    printf '%s: %s\n' "${0##*/}" "Implementation step not found: $step_file" >&2
    exit 66
fi
testing_file="$goal_dir/steps/${step_name}-testing.md"
if [ -e "$testing_file" ] && [ "$overwrite" = false ]; then
    printf '%s: %s\n' "${0##*/}" \
        "Testing companion already exists: $testing_file (pass --overwrite to replace it)" >&2
    exit 73
fi
[ -n "${instructions//[[:space:]]/}" ] || plan_die "Verification instructions must not be empty"
[[ "$instructions" != *'|'* ]] || plan_die "Verification instructions must not contain a Markdown table separator (|)"
[[ "$instructions" != *'§'* ]] || plan_die "Verification instructions must not contain the reserved paragraph marker §"

# Literal "\n" escapes and real newlines both separate paragraphs; each needs its
# own § 2.x label or it silently receives edits aimed at the labeled one. A regex
# split separator keeps mawk and BSD awk agreeing; a multi-char RS does not.
# ---- quoted: split separators ----
# /\\n/   literal backslash-n
# /\n+/   runs of real newlines
# ---- end quoted ----
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
printf 'Created %s\n' "$testing_file"