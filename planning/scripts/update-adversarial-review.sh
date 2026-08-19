#!/usr/bin/env bash
# update-adversarial-review.sh — rewrite the "## Findings" table in a plan's
# adversarial-review.md from CSV rows (ID, Missing or over-broad item, Required
# plan change, Status, Work unit).
#
# Reads CSV from adversarial-review-incoming.md when that file exists (the one
# plan file a reviewer may write, so a reviewer report survives the
# coordinator's context), else from --file PATH, else from stdin
# (heredoc-friendly). It rewrites only Findings; the Verdict is authored by the
# reviewer and flipped to approved via update-plan-content.sh --review-status.
#
# Usage:
#   update-adversarial-review.sh [--plan-dir] <plan-directory> [--file CSV] [--cycle N]
#   update-adversarial-review.sh --help
#
# The Work unit column is mandatory-with-blank-allowed: leave it empty (or N/A)
# for findings that carry no fix key, or name the owning work unit (WNN) to gate
# the finding. Cells must not contain `|`, and input must be LF, not CRLF.
#
# The previous Findings table is archived into adversarial-review-history.md
# under a `## Cycle N` heading (--cycle numbers it; otherwise the highest
# recorded number plus one) so reviewers of later cycles can see what earlier
# ones found. Archiving the same rows twice is a no-op; a --cycle that names an
# already-recorded cycle while holding different rows is refused rather than
# dropping them.
#
# Exit codes: 64 bad invocation or unusable CSV, 66 the plan or its review file
# is missing, 73 --cycle collides with a recorded cycle holding other findings.

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

source "$script_dir/plan-reconcile-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> [--file CSV] [--cycle N]
       ${0##*/} --help

Rewrites the adversarial-review "## Findings" table from CSV rows whose columns
are: ID, Missing or over-broad item, Required plan change, Status, Work unit.
Rows are read from adversarial-review-incoming.md if present, else from --file
CSV, else from stdin.

  --file CSV   read the rows from CSV instead of stdin
  --cycle N    number the archived history entry N instead of the next one up

This does not set the Verdict to approved. Author the Verdict and run
\`update-plan-content.sh --review-status <plan> approved\` separately.
USAGE
    exit "$rc"
}

# The highest `## Cycle N` recorded, 0 when there is none. It is a maximum and
# never a count: one explicit --cycle N above the count would otherwise send
# every later automatic number backwards, into labels already in use.
highest_cycle_number() {
    local history="$1" highest
    [ -f "$history" ] || { printf '0\n'; return 0; }
    highest="$({ grep -E '^## Cycle [0-9]+$' "$history" || true; } | awk '{ print $3 }' | sort -n | tail -1)"
    printf '%d\n' "${highest:-0}"
}

# The rows filed under the last heading — comparing them is what stops the same
# review being archived twice.
last_archived_rows() {
    local history="$1"
    [ -f "$history" ] || return 0
    awk '
        /^## Cycle [0-9]+$/ { rows = ""; next }
        /^\|/ { rows = rows $0 "\n" }
        END { printf "%s", rows }
    ' "$history"
}

cycle_already_recorded() {
    local history="$1" wanted="$2"
    [ -f "$history" ] || return 1
    awk -v wanted="$wanted" '
        /^## Cycle [0-9]+$/ && $3 + 0 == wanted + 0 { found = 1 }
        END { exit found ? 0 : 1 }
    ' "$history"
}

plan_dir=""
csv_source=""
consumed_incoming=0
cycle_number=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --file) [ "$#" -ge 2 ] || usage; csv_source="$2"; shift 2 ;;
        --cycle) [ "$#" -ge 2 ] || usage; cycle_number="$2"; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) [ -z "$plan_dir" ] || usage; plan_dir="$1"; shift ;;
    esac
done
[ -n "$plan_dir" ] || usage

plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
review_file="$plan_dir/adversarial-review.md"
[ -f "$review_file" ] || plan_die "adversarial-review.md not found: $review_file (run create-adversarial-review.sh first)" 66

# The trap is installed before the first write and never released with
# `trap - EXIT`: releasing it discards the library's cleanup handler (§8) and
# leaked both mktemp files on every successful run.
csv_file="$(mktemp "${TMPDIR:-/tmp}/adversarial-review-table.XXXXXX")"
rendered_file="$(mktemp "${TMPDIR:-/tmp}/adversarial-review-rendered.XXXXXX")"
temporary_file="${review_file}.tmp.$$"
trap 'rm -f "$csv_file" "$rendered_file" "$temporary_file"' EXIT

# Reviewers write their findings to adversarial-review-incoming.md; this is its
# only consumer. It takes precedence over stdin so a reviewer report survives
# the coordinator's context.
incoming_file="$plan_dir/adversarial-review-incoming.md"
if [ -n "$csv_source" ]; then
    [ -f "$csv_source" ] || plan_die "CSV file not found: $csv_source" 66
    cp "$csv_source" "$csv_file"
elif [ -f "$incoming_file" ]; then
    cp "$incoming_file" "$csv_file"
    consumed_incoming=1
    printf 'Consumed reviewer findings from %s\n' "$incoming_file" >&2
else
    if [ -t 0 ]; then
        plan_die "no CSV provided. Pipe rows or a heredoc to stdin, pass --file PATH, or let reviewers write adversarial-review-incoming.md (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)" 64
    fi
    cat > "$csv_file"
fi
[ -s "$csv_file" ] || plan_die "CSV input is empty (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)" 65

plan_render_csv_table 5 "$(awk '{ printf "%s\\n", $0 }' "$csv_file")" > "$rendered_file"

# Archive the prior Findings rows (if any) into adversarial-review-history.md
# so reviewers of later cycles can see what earlier ones found.
history_file="$plan_dir/adversarial-review-history.md"
history_rows="$(awk '
    /^## Findings$/ { in_findings = 1; next }
    in_findings && /^## Verdict$/ { exit }
    in_findings && /^\|/ { print }
' "$review_file")"
# Record a cycle entry on every rewrite, even with no rows to archive: the marker
# itself is the history a later reviewer needs. Comparing the last archived row
# set stops a double archive; an unarchived set is never dropped for a number.
if [ -z "$cycle_number" ]; then
    cycle_number=$(($(highest_cycle_number "$history_file") + 1))
fi
if [ -n "$history_rows" ] && [ "$history_rows" = "$(last_archived_rows "$history_file")" ]; then
    printf 'Findings table is already the last entry in %s; not archiving it twice\n' "$history_file" >&2
elif cycle_already_recorded "$history_file" "$cycle_number"; then
    plan_die "Cycle $cycle_number is already recorded in $history_file with other findings; archiving would discard them (choose a free --cycle number)" 73
else
    {
        printf '\n## Cycle %s\n\n' "$cycle_number"
        if [ -n "$history_rows" ]; then
            printf '%s\n' "$history_rows"
        else
            printf '%s\n' '_No row-level findings were recorded for this cycle._'
        fi
    } >> "$history_file"
    printf 'Archived previous Findings table to %s (Cycle %s)\n' "$history_file" "$cycle_number" >&2
fi

awk -v replacement_file="$rendered_file" '
    function emit_replacement(    line) {
        while ((getline line < replacement_file) > 0) print line
        close(replacement_file)
    }
    /^## Findings$/ {
        print
        print ""
        emit_replacement()
        in_findings = 1
        next
    }
    in_findings && /^## Verdict$/ {
        print ""
        print
        in_findings = 0
        next
    }
    in_findings { next }
    { print }
' "$review_file" > "$temporary_file"
mv "$temporary_file" "$review_file"
# A reviewer report only survives the coordinator's context while the file does,
# so it is removed only when it was the source that got rendered.
[ "$consumed_incoming" -eq 0 ] || rm -f "$incoming_file"
# Fix keys are re-minted from the rewritten table; its progress line is this
# script's diagnostic, not its result, so it goes to stderr (§10).
"$script_dir/mint-fix-keys.sh" "$plan_dir" >&2

printf 'Updated findings table in %s\n' "$review_file"
