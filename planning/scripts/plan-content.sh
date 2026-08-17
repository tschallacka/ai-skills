#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
Usage:
  plan-content.sh get <plan-directory> <document-id> [markdown|text|json|path]
  plan-content.sh summary <plan-directory> [markdown|text|json]
  plan-content.sh blast-radius <plan-directory> <WNN|goal-name|goal-name/step-name> [markdown|text|json]
  plan-content.sh find <plan-directory> <pattern> [--in plan|goals|steps|units|review|all] [--format text|json]
                                    literal search; prints docid<TAB>section<TAB>excerpt per match,
                                    exits 1 on zero or multiple matches
USAGE
    exit 64
}

[ "$#" -ge 1 ] || usage
command="$1"; shift
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

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

collect_units() {
    awk -F'|' '
        function trim(value) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", value); gsub(/^`|`$/, "", value); return value }
        /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
            print trim($2) "\t" trim($3) "\t" trim($4) "\t" trim($5) "\t" trim($6) "\t" trim($7) "\t" trim($8) "\t" trim($9) "\t" trim($10)
        }
    ' "$1/work-unit-inventory.md"
}

case "$command" in
    get)
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || { printf 'plan-content.sh: get requires <plan-directory> <document-id> [markdown|text|json|path]\n' >&2; exit 64; }
        plan_dir="$1"; document_id="$2"; format="${3:-markdown}"
        plan_require_directory "$plan_dir"
        file="$(plan_document_path "$plan_dir" "$document_id")"
        [ -f "$file" ] || plan_die "Document not found: $file"
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
                collect_units "$plan_dir" | awk -F'\t' '{ printf "| %s | %s | %s | %s | %s | %s | %s |\n", $1, $2, $3, $4, $7, $8, $9 }'
                ;;
            text)
                collect_units "$plan_dir" | awk -F'\t' '{ printf "%s  %s  %s :: %s  <- %s  [%s/%s]\n", $1, $2, $3, $4, $7, $8, $9 }'
                ;;
            json)
                printf '{"plan":"%s","work_units":[' "$(basename "$plan_dir")"
                first=true
                while IFS=$'\t' read -r id type file scope subscope intended depends goal step; do
                    [ "$first" = true ] || printf ','
                    first=false
                    printf '{"id":"%s","type":"%s","file":"%s","scope":"%s","depends_on":"%s","goal":"%s","step":"%s"}' "$id" "$type" "$file" "$scope" "$depends" "$goal" "$step"
                done < <(collect_units "$plan_dir")
                printf ']}\n'
                ;;
            *) plan_die "Unknown format: $format (use markdown, text, or json)" ;;
        esac
        ;;
    blast-radius)
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage
        plan_dir="$1"; target="$2"; format="${3:-markdown}"
        plan_require_directory "$plan_dir"
        inventory="$plan_dir/work-unit-inventory.md"
        [ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
        declare -A unit_goal unit_step unit_depends selected impacted
        while IFS=$'\t' read -r id type file scope subscope intended depends goal step; do
            unit_goal[$id]="$goal"; unit_step[$id]="$step"; unit_depends[$id]="$depends"
        done < <(collect_units "$plan_dir")
        case "$target" in
            W*) [ -n "${unit_goal[$target]+x}" ] || plan_die "Work unit not found: $target"; selected[$target]=1 ;;
            */*)
                target_goal="${target%%/*}"; target_step="${target#*/}"; found=false
                for id in "${!unit_goal[@]}"; do
                    if [ "${unit_goal[$id]}" = "$target_goal" ] && [ "${unit_step[$id]}" = "$target_step" ]; then selected[$id]=1; found=true; fi
                done
                [ "$found" = true ] || plan_die "Step not found: $target"
                ;;
            *)
                found=false
                for id in "${!unit_goal[@]}"; do
                    if [ "${unit_goal[$id]}" = "$target" ]; then selected[$id]=1; found=true; fi
                done
                [ "$found" = true ] || plan_die "Goal not found: $target"
                ;;
        esac
        changed=true
        while [ "$changed" = true ]; do
            changed=false
            for id in "${!unit_goal[@]}"; do
                for dependency in ${unit_depends[$id]//,/ }; do
                    if [ -n "${selected[$dependency]+x}" ] && [ -z "${impacted[$id]+x}" ] && [ -z "${selected[$id]+x}" ]; then
                        impacted[$id]=1; changed=true
                    fi
                    if [ -n "${impacted[$dependency]+x}" ] && [ -z "${impacted[$id]+x}" ] && [ -z "${selected[$id]+x}" ]; then
                        impacted[$id]=1; changed=true
                    fi
                done
            done
        done
        case "$format" in
            markdown)
                printf '# Blast radius: %s\n\n' "$target"
                printf '## Changed\n\n'
                for id in "${!selected[@]}"; do printf -- '- `%s` → `%s/%s`\n' "$id" "${unit_goal[$id]}" "${unit_step[$id]}"; done | sort
                printf '\n## Downstream work units\n\n'
                if [ "${#impacted[@]}" -eq 0 ]; then printf '%s\n' '- None'; else for id in "${!impacted[@]}"; do printf -- '- `%s` → `%s/%s`\n' "$id" "${unit_goal[$id]}" "${unit_step[$id]}"; done | sort; fi
                ;;
            text)
                for id in "${!selected[@]}"; do printf 'changed %s -> %s/%s\n' "$id" "${unit_goal[$id]}" "${unit_step[$id]}"; done | sort
                for id in "${!impacted[@]}"; do printf 'downstream %s -> %s/%s\n' "$id" "${unit_goal[$id]}" "${unit_step[$id]}"; done | sort
                ;;
            json)
                printf '{"target":"%s","changed":[' "$target"; first=true
                for id in "${!selected[@]}"; do [ "$first" = true ] || printf ','; first=false; printf '"%s"' "$id"; done
                printf '],"downstream":['; first=true
                for id in "${!impacted[@]}"; do [ "$first" = true ] || printf ','; first=false; printf '"%s"' "$id"; done
                printf ']}\n'
                ;;
            *) plan_die "Unknown format: $format (use markdown, text, or json)" ;;
        esac
        ;;
    find)
        [ "$#" -ge 2 ] || usage
        plan_dir="$1"; pattern="$2"; shift 2
        scope=all; format=text
        while [ "$#" -gt 0 ]; do
            case "$1" in
                --in) [ "$#" -ge 2 ] || usage; scope="$2"; shift 2 ;;
                --format) [ "$#" -ge 2 ] || usage; format="$2"; shift 2 ;;
                -h|--help) usage ;;
                -*) usage ;;
                *) usage ;;
            esac
        done
        plan_require_directory "$plan_dir"
        case "$scope" in plan|goals|steps|units|review|all) ;; *) plan_die "Unknown scope: $scope (use plan, goals, steps, units, review, or all)" ;; esac
        case "$format" in text|json) ;; *) plan_die "Unknown format: $format (use text or json)" ;; esac
        matches_file="$(mktemp "${TMPDIR:-/tmp}/plan-find.XXXXXX")"
        trap 'rm -f "$matches_file"' EXIT
        scan_file() {
            local docid="$1" file="$2"
            [ -f "$file" ] || return 0
            awk -v docid="$docid" -v pattern="$pattern" '
                $0 ~ /^§ [0-9]+\.[0-9]+[[:space:]]*$/ { last = $2; next }
                index($0, pattern) {
                    line = $0
                    sub(/^[[:space:]]*/, "", line)
                    if (length(line) > 120) line = substr(line, 1, 120) "..."
                    print docid "\t" (last ? last : "-") "\t" line
                }
            ' "$file"
        }
        case "$scope" in
            plan|all) scan_file 'plan' "$plan_dir/plan-description.md" >> "$matches_file" ;;
        esac
        case "$scope" in
            review|all) scan_file 'review' "$plan_dir/adversarial-review.md" >> "$matches_file" ;;
        esac
        case "$scope" in
            goals|all)
                for goal_file in "$plan_dir"/*/goal.md; do
                    [ -f "$goal_file" ] || continue
                    scan_file "goal:$(basename "$(dirname "$goal_file")")" "$goal_file" >> "$matches_file"
                done
                ;;
        esac
        case "$scope" in
            steps|all)
                for step_file in "$plan_dir"/*/steps/*.md; do
                    [ -f "$step_file" ] || continue
                    [[ "$(basename "$step_file")" == *-testing.md ]] && continue
                    goal_name="$(basename "$(dirname "$(dirname "$step_file")")")"
                    step_name="$(basename "$step_file" .md)"
                    scan_file "step:$goal_name/$step_name" "$step_file" >> "$matches_file"
                done
                ;;
        esac
        case "$scope" in
            units|all)
                [ -f "$plan_dir/work-unit-inventory.md" ] && awk -F'|' -v pattern="$pattern" '
                    /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
                        id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
                        if (index($0, pattern)) {
                            row = $0; gsub(/^[[:space:]]*\|[[:space:]]*/, "", row); gsub(/[[:space:]]*\|[[:space:]]*$/, "", row)
                            if (length(row) > 120) row = substr(row, 1, 120) "..."
                            print "unit:" id "\t" row "\t" row
                        }
                    }
                ' "$plan_dir/work-unit-inventory.md" >> "$matches_file"
                ;;
        esac
        if [ "$format" = json ]; then
            printf '{"matches":['
            first=true
            while IFS=$'\t' read -r docid section excerpt; do
                [ "$first" = true ] || printf ','
                first=false
                printf '{"document":"%s","section":"%s","excerpt":"%s"}' \
                    "$(printf '%s' "$docid" | awk '{ gsub(/\\/, "\\\\"); gsub(/\"/, "\\\""); print }')" \
                    "$(printf '%s' "$section" | awk '{ gsub(/\\/, "\\\\"); gsub(/\"/, "\\\""); print }')" \
                    "$(printf '%s' "$excerpt" | awk '{ gsub(/\\/, "\\\\"); gsub(/\"/, "\\\""); print }')"
            done < "$matches_file"
            printf ']}\n'
        else
            cat "$matches_file"
        fi
        match_count="$(wc -l < "$matches_file")"
        rm -f "$matches_file"
        trap - EXIT
        if [ "$match_count" -eq 0 ]; then
            printf 'plan-content.sh: find: no matches for %s (scope: %s)\n' "$pattern" "$scope" >&2
            exit 1
        fi
        if [ "$match_count" -gt 1 ]; then
            printf 'plan-content.sh: find: %s matches for %s (scope: %s); narrow the pattern or scope to get a single hit\n' "$match_count" "$pattern" "$scope" >&2
            exit 1
        fi
        ;;
    *) usage ;;
esac
