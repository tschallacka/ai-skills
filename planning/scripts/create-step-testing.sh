#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# create-step-testing.sh — create or (with --overwrite) replace a step's
# testing companion. Input is validated BEFORE any filesystem change, so a
# rejected call never leaves the plan with the old companion already deleted.
#
# Usage:
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions>
#   create-step-testing.sh <goal-directory> <step-name> <verification-instructions> --overwrite
#   create-step-testing.sh --help
#
# The positional instructions are rendered as a numbered §2.x section under
# "## Automated tests"; separate paragraphs with "\n" escapes or real newlines
# (multi-paragraph companions are supported and are what reviewers actually
# proofread; every paragraph gets its own § 2.N label).
#
# --browser, --backend and --manual add the other verification sections. A
# section number is fixed per section name, not by position: automated tests are
# always §2.x, browser §3.x, backend §4.x, manual §5.x, whichever sections a
# companion happens to carry. That is what makes `update-plan-content.sh -ss`
# able to address them -- and until these flags existed it advertised three
# sections that no helper could create.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <goal-directory> <step-name> <verification-instructions>
              [--browser TEXT] [--backend TEXT] [--manual TEXT] [--overwrite]
       ${0##*/} --help

  <verification-instructions>  automated tests, section 2
  --browser TEXT               browser verification, section 3
  --backend TEXT               backend verification, section 4
  --manual TEXT                manual verification, section 5

Paragraphs split on "\\n" escapes or real newlines; each gets its own label.
USAGE
    exit "$rc"
}

# A flag loop, not a "$@" pre-scan into an array: -h belongs in band, and
# re-setting "$@" from a possibly-empty array aborts under set -u on bash 3.2.
overwrite=false
browser_text=""
backend_text=""
manual_text=""
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --overwrite) overwrite=true; shift ;;
        --browser) [ "$#" -ge 2 ] || usage; browser_text="$2"; shift 2 ;;
        --backend) [ "$#" -ge 2 ] || usage; backend_text="$2"; shift 2 ;;
        --manual) [ "$#" -ge 2 ] || usage; manual_text="$2"; shift 2 ;;
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
# Every supplied section is checked before anything is written, so a rejected
# --manual never leaves a companion holding only the sections that came first.
require_valid_instructions() { # <what> <text>
    local what="$1" text="$2"
    [ -n "${text//[[:space:]]/}" ] || plan_die "$what instructions must not be empty"
    [[ "$text" != *'|'* ]] || plan_die "$what instructions must not contain a Markdown table separator (|)"
    [[ "$text" != *'§'* ]] || plan_die "$what instructions must not contain the reserved paragraph marker §"
}
require_valid_instructions 'Verification' "$instructions"
[ -z "$browser_text" ] || require_valid_instructions 'Browser verification' "$browser_text"
[ -z "$backend_text" ] || require_valid_instructions 'Backend verification' "$backend_text"
[ -z "$manual_text" ] || require_valid_instructions 'Manual verification' "$manual_text"

# Literal "\n" escapes and real newlines both separate paragraphs; each needs its
# own § 2.x label or it silently receives edits aimed at the labeled one. A regex
# split separator keeps mawk and BSD awk agreeing; a multi-char RS does not.
# ---- quoted: split separators ----
# /\\n/   literal backslash-n
# /\n+/   runs of real newlines
# ---- end quoted ----
bodies_dir="$(mktemp -d "${TMPDIR:-/tmp}/plan-testing.XXXXXX")"
trap 'rm -rf "$bodies_dir"' EXIT

# The section number is a parameter because it is fixed per section name: a
# companion carrying only browser content still labels it § 3.x, so -ss and -sp
# address the same labels whatever sections exist.
render_section_body() { # <text> <section-number> <output-file> <what>
    printf '%s' "$1" | awk -v section="$2" '
        { text = text $0 "\n" }
        END {
            gsub(/\\n/, "\n", text)
            n = split(text, paras, /\n+/)
            for (i = 1; i <= n; i++) {
                para = paras[i]
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", para)
                if (para == "") continue
                if (count++) printf "\n\n"
                printf "§ %d.%d\n%s", section, count, para
            }
            if (count == 0) exit 1
        }
    ' > "$3" || plan_die "$4 instructions are empty after splitting"
}

render_section_body "$instructions" 2 "$bodies_dir/automated" 'Verification'
[ -z "$browser_text" ] || render_section_body "$browser_text" 3 "$bodies_dir/browser" 'Browser verification'
[ -z "$backend_text" ] || render_section_body "$backend_text" 4 "$bodies_dir/backend" 'Backend verification'
[ -z "$manual_text" ] || render_section_body "$manual_text" 5 "$bodies_dir/manual" 'Manual verification'

temporary_file="${testing_file}.tmp.$$"
trap 'rm -rf "$bodies_dir" "$temporary_file"' EXIT
{
    printf '# Verification: %s\n\n' "$step_name"
    printf '## Automated tests\n\n'
    cat "$bodies_dir/automated"
    [ ! -f "$bodies_dir/browser" ] || { printf '\n\n## Browser verification\n\n'; cat "$bodies_dir/browser"; }
    [ ! -f "$bodies_dir/backend" ] || { printf '\n\n## Backend verification\n\n'; cat "$bodies_dir/backend"; }
    [ ! -f "$bodies_dir/manual" ] || { printf '\n\n## Manual verification\n\n'; cat "$bodies_dir/manual"; }
} > "$temporary_file"
mv "$temporary_file" "$testing_file"
printf 'Created %s\n' "$testing_file"