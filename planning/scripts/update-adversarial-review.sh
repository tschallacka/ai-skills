#!/usr/bin/env bash
# update-adversarial-review.sh — rewrite the "## Findings" table in a plan's
# adversarial-review.md from CSV rows (ID, Missing or over-broad item, Required
# plan change, Status, Work unit). Reads CSV from stdin by default
# (heredoc-friendly) or from --file PATH. It only rewrites Findings; the
# Verdict is authored by the reviewer and flipped to approved via
# update-plan-content.sh --review-status.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/plan-document-lib.sh"
source "$SCRIPT_DIR/plan-reconcile-lib.sh"

readonly HELP=$'Updates the adversarial-review "## Findings" table.\n\nUsage: update-adversarial-review.sh <plan-directory> [--file CSV] [--cycle N]\n\nReads CSV rows (ID, Missing or over-broad item, Required plan change, Status,\nWork unit) from stdin by default (heredoc-friendly), or from --file PATH, and\nrewrites only the Findings section of adversarial-review.md. Work unit is\nmandatory-with-blank-allowed: leave it empty (or N/A) for findings that carry\nno fix key, or name the owning work unit (WNN) to gate the finding.\n\nInput rules: cells must not contain `|`; input must be LF, not CRLF (no\nWindows line endings).\n\nThe previous Findings table is archived into adversarial-review-history.md\nunder a `## Cycle N` heading (use --cycle to number it; the next free number is\nused otherwise) so reviewers of later cycles can see what earlier ones found.\n\nNote: this does not set the Verdict to approved. Author the Verdict and run\n`update-plan-content.sh --review-status <plan> approved` separately.'

case "${1:-}" in
    -h|--help) printf '%s\n' "$HELP"; exit 0 ;;
esac
[ "$#" -ge 1 ] && [ "$#" -le 5 ] || plan_err "usage: update-adversarial-review.sh <plan-directory> [--file CSV] [--cycle N] (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"

plan_dir="$1"; shift
csv_source=""
cycle_number=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --file)
            [ "$#" -ge 2 ] || plan_err "usage: update-adversarial-review.sh <plan-directory> [--file CSV] [--cycle N] (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"
            csv_source="$2"; shift 2
            ;;
        --cycle)
            [ "$#" -ge 2 ] || plan_err "--cycle requires a number"
            cycle_number="$2"; shift 2
            ;;
        *) plan_err "usage: update-adversarial-review.sh <plan-directory> [--file CSV] [--cycle N] (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)" ;;
    esac
done

plan_require_directory "$plan_dir" || plan_err "plan directory not found: $plan_dir (pass an absolute path, e.g. .plans/<plan-name>)"
plan_git_snapshot "$plan_dir"
review_file="$plan_dir/adversarial-review.md"
[ -f "$review_file" ] || plan_err "adversarial-review.md not found: $review_file (run create-adversarial-review.sh first)"

csv_file="$(mktemp "${TMPDIR:-/tmp}/adversarial-review-table.XXXXXX")"
rendered_file="$(mktemp "${TMPDIR:-/tmp}/adversarial-review-rendered.XXXXXX")"
temporary_file="${review_file}.tmp.$$"
trap 'rm -f "$csv_file" "$rendered_file" "$temporary_file"' EXIT

# Reviewers write their findings to adversarial-review-incoming.md (the one
# plan file a reviewer may write); this is its only consumer. It takes
# precedence over stdin so a reviewer report survives the coordinator's context.
incoming_file="$plan_dir/adversarial-review-incoming.md"
if [ -n "$csv_source" ]; then
    [ -f "$csv_source" ] || plan_err "CSV file not found: $csv_source"
    cp "$csv_source" "$csv_file"
elif [ -f "$incoming_file" ]; then
    cp "$incoming_file" "$csv_file"
    printf 'Consumed reviewer findings from %s\n' "$incoming_file"
else
    if [ -t 0 ]; then
        plan_err "no CSV provided. Pipe rows or a heredoc to stdin, pass --file PATH, or let reviewers write adversarial-review-incoming.md (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"
    fi
    cat > "$csv_file"
fi
[ -s "$csv_file" ] || plan_err "CSV input is empty (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"

plan_render_csv_table 5 "$(awk '{ printf "%s\\n", $0 }' "$csv_file")" > "$rendered_file"

# Archive the prior Findings rows (if any) into adversarial-review-history.md
# so reviewers of later cycles can see what earlier ones found.
history_file="$plan_dir/adversarial-review-history.md"
history_rows="$(awk '
    /^## Findings$/ { in_findings = 1; next }
    in_findings && /^## Verdict$/ { exit }
    in_findings && /^\|/ { print }
' "$review_file")"
# Record a cycle entry whenever a rewrite happens, even when the prior table
# was replaced by narrative (no row-level rows to archive) — the cycle marker
# itself is the history a later reviewer needs. Guard against re-archiving the
# same review file twice by comparing the last archived row set.
if [ -z "$cycle_number" ]; then
    cycle_number="$(grep -c '^## Cycle ' "$history_file" 2>/dev/null || true)"
    cycle_number=$((cycle_number + 1))
fi
last_cycle="$(grep -oE '^## Cycle [0-9]+' "$history_file" 2>/dev/null | tail -1 | awk '{print $3}')" || true
if [ "$last_cycle" != "$cycle_number" ]; then
    {
        printf '\n## Cycle %s\n\n' "$cycle_number"
        if [ -n "$history_rows" ]; then
            printf '%s\n' "$history_rows"
        else
            printf '%s\n' '_No row-level findings were recorded for this cycle._'
        fi
    } >> "$history_file"
    printf 'Archived previous Findings table to %s (Cycle %s)\n' "$history_file" "$cycle_number"
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
[ -f "$incoming_file" ] && rm -f "$incoming_file"
"$SCRIPT_DIR/mint-fix-keys.sh" "$plan_dir"
trap - EXIT
printf 'Updated findings table in %s\n' "$review_file"