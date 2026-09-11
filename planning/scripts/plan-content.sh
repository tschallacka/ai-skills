#!/usr/bin/env bash
# MODE: PROD
# plan-content.sh — read-only queries over one plan's documents.
#
# Five subcommands, all non-mutating: `get` prints one document, `summary`
# renders the work-unit inventory, `blast-radius` walks the depends-on graph
# from a unit/goal/step, `find` does a literal single-hit search that exits 1
# unless exactly one line matches, and `diff` maps changed lines since a git ref
# back to the enclosing "§ N.N" paragraph labels.
#
# Usage:
#   The plan directory may be given positionally or as --plan-dir <path>.
#   plan-content.sh get|summary|blast-radius|find|diff [--plan-dir] <plan-directory> [...]
#   plan-content.sh --help
#
# Exit codes: 1 zero or multiple `find` matches, 64 bad invocation, 66 missing
# document.
# shellcheck disable=SC2154  # plan_inventory_* are assigned at runtime by the
# sourced plan-inventory-lib row/split helpers

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage:
  ${0##*/} get [--plan-dir] <plan-directory> <document-id> [markdown|text|json|path]
  ${0##*/} summary [--plan-dir] <plan-directory> [markdown|text|json]
  ${0##*/} blast-radius [--plan-dir] <plan-directory> <WNN|goal-name|goal-name/step-name> [markdown|text|json]
  ${0##*/} find [--plan-dir] <plan-directory> <pattern> [--in plan|goals|steps|units|review|testing|coverage|stories|all] [--document <docid>] [--full] [--format text|json]
                                    literal search; prints docid<TAB>section<TAB>excerpt per match,
                                    exits 1 on zero or multiple matches; --document scopes to one document
                                    (plan, review, coverage, stories, planning-bugs, goal:<g>, step:<g>/<s>, unit:<WNN>, or a
                                    step:-testing id); --full disables excerpt truncation
  ${0##*/} diff [--plan-dir] <plan-directory> <git-ref> [--format text|json]
                                    lists documents changed since git-ref and the
                                    paragraph labels touched in each
USAGE
    exit "$rc"
}

[ "$#" -ge 1 ] || usage
if [ "$1" = '-h' ] || [ "$1" = '--help' ]; then
    usage 0
fi
command="$1"; shift
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory (the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here).
# The subcommand was already shifted off above, so the plan directory is $1.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

json_string_file() {
    awk 'BEGIN { printf "\"" } { gsub(/\\/, "\\\\"); gsub(/\"/, "\\\""); if (NR > 1) printf "\\n"; printf "%s", $0 } END { printf "\"" }' "$1"
}

format_document() {
    local format="$1" id="$2" file="$3"
    case "$format" in
        markdown) cat "$file" ;;
        text) printf 'Document: %s\nPath: %s\n\n' "$id" "$file"; cat "$file" ;;
        path) printf '%s\n' "$file" ;;
        json) printf '{"id":"%s","path":"%s","content":' "$id" "$file"; json_string_file "$file"; printf '}\n' ;;
        *) plan_die "Unknown format: $format (use markdown, text, json, or path)" ;;
    esac
}

case "$command" in
    get)
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || { printf 'plan-content.sh: get requires [--plan-dir] <plan-directory> <document-id> [markdown|text|json|path]\n' >&2; exit 64; }
        plan_dir="$1"; document_id="$2"; format="${3:-markdown}"
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file" 66
        format_document "$format" "$document_id" "$file"
        ;;
    summary)
        [ "$#" -ge 1 ] && [ "$#" -le 2 ] || usage
        plan_dir="$1"; format="${2:-markdown}"
        plan_require_directory "$plan_dir"
        inventory="$plan_dir/work-unit-inventory.md"
        [ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
        case "$format" in
            markdown)
                printf '# Plan summary: %s\n\n' "$(basename "$plan_dir")"
                printf '| ID | Type | File | Scope | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|\n'
                while IFS= read -r row; do
                    plan_inventory_split "$row"
                    printf '| %s | %s | %s | %s | %s | %s | %s |\n' \
                        "$plan_inventory_id" "$plan_inventory_type" "$plan_inventory_file" \
                        "$plan_inventory_scope" "$plan_inventory_depends" \
                        "$plan_inventory_goal" "$plan_inventory_step"
                done < <(plan_inventory_rows "$inventory")
                ;;
            text)
                while IFS= read -r row; do
                    plan_inventory_split "$row"
                    printf '%s  %s  %s :: %s  <- %s  [%s/%s]\n' \
                        "$plan_inventory_id" "$plan_inventory_type" "$plan_inventory_file" \
                        "$plan_inventory_scope" "$plan_inventory_depends" \
                        "$plan_inventory_goal" "$plan_inventory_step"
                done < <(plan_inventory_rows "$inventory")
                ;;
            json)
                printf '{"plan":"%s","work_units":[' "$(basename "$plan_dir")"
                first=true
                while IFS=$'\t' read -r id type file scope subscope intended depends goal step; do
                    [ "$first" = true ] || printf ','
                    first=false
                    printf '{"id":"%s","type":"%s","file":"%s","scope":"%s","depends_on":"%s","goal":"%s","step":"%s"}' "$id" "$type" "$file" "$scope" "$depends" "$goal" "$step"
                done < <(plan_inventory_rows "$inventory")
                printf ']}\n'
                ;;
            *) plan_die "Unknown format: $format (use markdown, text, or json)" ;;
        esac
        ;;
    blast-radius)
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage
        plan_dir="$1"; target="$2"; format="${3:-markdown}"
        plan_require_directory "$plan_dir"
        # This answers "which units depend on this one", not "where is it
        # mentioned" — and the plausible half answer is where readers stop
        # (T13). Point at the document sweep before the output begins.
        printf '%s\n' "plan-content.sh: blast-radius walks dependency edges. To sweep every document mentioning this unit, run: $0 find --plan-dir <plan-directory> \"$target\" --in all" >&2
        inventory="$plan_dir/work-unit-inventory.md"
        [ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
        # bash 3.2 has no associative arrays, so these are plan_map_* maps.
        # Cleared first: this subcommand can run twice in one process via
        # plan-mutate.sh.
        plan_map_clear unit_goal; plan_map_clear unit_step; plan_map_clear unit_depends
        plan_map_clear selected; plan_map_clear impacted
        while IFS=$'\t' read -r id type file scope subscope intended depends goal step; do
            plan_map_set unit_goal "$id" "$goal"
            plan_map_set unit_step "$id" "$step"
            plan_map_set unit_depends "$id" "$depends"
        done < <(plan_inventory_rows "$inventory")
        case "$target" in
            W*) plan_map_has unit_goal "$target" || plan_die "Work unit not found: $target"; plan_map_set selected "$target" 1 ;;
            */*)
                target_goal="${target%%/*}"; target_step="${target#*/}"; found=false
                while IFS= read -r id; do
                    [ -n "$id" ] || continue
                    plan_map_load unit_goal "$id" || plan_map_value=""
                    [ "$plan_map_value" = "$target_goal" ] || continue
                    plan_map_load unit_step "$id" || plan_map_value=""
                    if [ "$plan_map_value" = "$target_step" ]; then plan_map_set selected "$id" 1; found=true; fi
                done < <(plan_map_keys unit_goal)
                [ "$found" = true ] || plan_die "Step not found: $target"
                ;;
            *)
                found=false
                while IFS= read -r id; do
                    [ -n "$id" ] || continue
                    plan_map_load unit_goal "$id" || plan_map_value=""
                    if [ "$plan_map_value" = "$target" ]; then plan_map_set selected "$id" 1; found=true; fi
                done < <(plan_map_keys unit_goal)
                [ "$found" = true ] || plan_die "Goal not found: $target"
                ;;
        esac
        changed=true
        while [ "$changed" = true ]; do
            changed=false
            while IFS= read -r id; do
                [ -n "$id" ] || continue
                plan_map_load unit_depends "$id" || plan_map_value=""
                for dependency in ${plan_map_value//,/ }; do
                    if plan_map_has selected "$dependency" && ! plan_map_has impacted "$id" && ! plan_map_has selected "$id"; then
                        plan_map_set impacted "$id" 1; changed=true
                    fi
                    if plan_map_has impacted "$dependency" && ! plan_map_has impacted "$id" && ! plan_map_has selected "$id"; then
                        plan_map_set impacted "$id" 1; changed=true
                    fi
                done
            done < <(plan_map_keys unit_goal)
        done
        # Render one map's rows through $1 as a printf template taking id, goal
        # and step. Keys come out in insertion order; markdown/text sort anyway.
        blast_rows() {            local map="$1" template="$2" id
            while IFS= read -r id; do
                [ -n "$id" ] || continue
                plan_map_load unit_goal "$id" || plan_map_value=""
                local row_goal="$plan_map_value"
                plan_map_load unit_step "$id" || plan_map_value=""
                # shellcheck disable=SC2059  # template is a caller-supplied format
                printf -- "$template" "$id" "$row_goal" "$plan_map_value"
            done < <(plan_map_keys "$map")
        }
        case "$format" in
            markdown)
                printf '# Blast radius: %s\n\n' "$target"
                printf '## Changed\n\n'
                blast_rows selected '- `%s` → `%s/%s`\n' | sort
                printf '\n## Downstream work units\n\n'
                if [ "$(plan_map_count impacted)" -eq 0 ]; then printf '%s\n' '- None'; else blast_rows impacted '- `%s` → `%s/%s`\n' | sort; fi
                ;;
            text)
                blast_rows selected 'changed %s -> %s/%s\n' | sort
                blast_rows impacted 'downstream %s -> %s/%s\n' | sort
                ;;
            json)
                # plan_map_keys yields insertion order; sorting is what pins
                # the JSON array to a deterministic order across bash builds.
                blast_ids() {
                    local first=true id
                    while IFS= read -r id; do
                        [ -n "$id" ] || continue
                        [ "$first" = true ] || printf ','
                        first=false
                        printf '"%s"' "$id"
                    done < <(plan_map_keys "$1" | sort)
                }
                printf '{"target":"%s","changed":[' "$target"
                blast_ids selected
                printf '],"downstream":['
                blast_ids impacted
                printf ']}\n'
                ;;
            *) plan_die "Unknown format: $format (use markdown, text, or json)" ;;
        esac
        ;;
    find)
        [ "$#" -ge 2 ] || usage
        # The find subcommand lives in a sibling library so this file stays
        # under the CODE-STYLE.md §3 size limit.
        source "$script_dir/plan-content-lib.sh"
        content_find_command "$@"
        ;;
    diff)
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage
        # The diff subcommand lives in a sibling library so this file stays
        # under the CODE-STYLE.md §3 size limit.
        source "$script_dir/plan-content-diff-lib.sh"
        plan_content_diff "$@"
        ;;
    *) usage ;;
esac
