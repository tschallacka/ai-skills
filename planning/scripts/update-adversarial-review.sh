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

readonly HELP=$'Updates the adversarial-review "## Findings" table.\n\nUsage: update-adversarial-review.sh <plan-directory> [--file CSV]\n\nReads CSV rows (ID, Missing or over-broad item, Required plan change, Status,\nWork unit) from stdin by default (heredoc-friendly), or from --file PATH, and\nrewrites only the Findings section of adversarial-review.md. Work unit is\nmandatory-with-blank-allowed: leave it empty (or N/A) for findings that carry\nno fix key, or name the owning work unit (WNN) to gate the finding.\n\nInput rules: cells must not contain `|`; input must be LF, not CRLF (no\nWindows line endings).\n\nNote: this does not set the Verdict to approved. Author the Verdict and run\n`update-plan-content.sh --review-status <plan> approved` separately.'

case "${1:-}" in
    -h|--help) printf '%s\n' "$HELP"; exit 0 ;;
esac
[ "$#" -ge 1 ] && [ "$#" -le 3 ] || plan_err "usage: update-adversarial-review.sh <plan-directory> [--file CSV] (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"

plan_dir="$1"; shift
csv_source=""
if [ "$#" -gt 0 ]; then
    [ "$1" = --file ] && [ "$#" -eq 2 ] || plan_err "usage: update-adversarial-review.sh <plan-directory> [--file CSV] (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"
    csv_source="$2"
fi

plan_require_directory "$plan_dir" || plan_err "plan directory not found: $plan_dir (pass an absolute path, e.g. .plans/<plan-name>)"
plan_git_snapshot "$plan_dir"
review_file="$plan_dir/adversarial-review.md"
[ -f "$review_file" ] || plan_err "adversarial-review.md not found: $review_file (run create-adversarial-review.sh first)"

csv_file="$(mktemp "${TMPDIR:-/tmp}/adversarial-review-table.XXXXXX")"
rendered_file="$(mktemp "${TMPDIR:-/tmp}/adversarial-review-rendered.XXXXXX")"
temporary_file="${review_file}.tmp.$$"
trap 'rm -f "$csv_file" "$rendered_file" "$temporary_file"' EXIT

if [ -n "$csv_source" ]; then
    [ -f "$csv_source" ] || plan_err "CSV file not found: $csv_source"
    cp "$csv_source" "$csv_file"
else
    if [ -t 0 ]; then
        plan_err "no CSV provided. Pipe rows or a heredoc to stdin, or pass --file PATH (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"
    fi
    cat > "$csv_file"
fi
[ -s "$csv_file" ] || plan_err "CSV input is empty (columns: ID, Missing or over-broad item, Required plan change, Status, Work unit)"

plan_render_csv_table 5 "$(awk '{ printf "%s\\n", $0 }' "$csv_file")" > "$rendered_file"

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
"$SCRIPT_DIR/mint-fix-keys.sh" "$plan_dir"
trap - EXIT
printf 'Updated findings table in %s\n' "$review_file"