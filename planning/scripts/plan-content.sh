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
        plan_dir="$1"; pattern="$2"; shift 2
        scope=all; format=text; document=""; full=false
        while [ "$#" -gt 0 ]; do
            case "$1" in
                --in) [ "$#" -ge 2 ] || usage; scope="$2"; shift 2 ;;
                --document) [ "$#" -ge 2 ] || usage; document="$2"; shift 2 ;;
                --full) full=true; shift ;;
                --format) [ "$#" -ge 2 ] || usage; format="$2"; shift 2 ;;
                -h|--help) usage ;;
                -*) usage ;;
                *) usage ;;
            esac
        done
        plan_require_directory "$plan_dir"
        case "$scope" in plan|goals|steps|units|review|testing|coverage|stories|all|inventory) ;; *) plan_die "Unknown scope: $scope (use plan, goals, steps, units, review, testing, coverage, stories, inventory, or all)" ;; esac
        case "$format" in text|json) ;; *) plan_die "Unknown format: $format (use text or json)" ;; esac
        # Literal scan of one document (used by both --document and the scoped
        # branches below; defined here so --document can call it).
        # Row emitters for table-bearing documents: bash read-loops over the
        # shared cell helper keep every consumer on one parsing contract.
        emit_row() { # DOCID LINE FULL — outer pipes stripped, 120-char excerpt
            local docid="$1" line="$2" row
            row="$(printf '%s\n' "$line" | sed -e 's/^[[:space:]]*|[[:space:]]*//' -e 's/[[:space:]]*|[[:space:]]*$//')"
            if [ "$full" != true ] && [ "${#row}" -gt 120 ]; then
                row="${row:0:120}..."
            fi
            printf '%s\t%s\t%s\n' "$docid" "$row" "$row"
        }
        scan_coverage_rows() { # FILE DOCID PATTERN FULL — DoD coverage section
            local cfile="$1" cdocid="$2" cpat="$3" cfull="$4" in_cov=0 cline
            while IFS= read -r cline || [ -n "$cline" ]; do
                case "$cline" in
                    '## Definition-of-done coverage'*) in_cov=1; continue ;;
                esac
                [ "$in_cov" = 1 ] || continue
                case "$cline" in '## '*) break ;; esac
                case "$cline" in
                    '|'*)
                        if [ -z "$cpat" ] || [[ $cline == *"$cpat"* ]]; then
                            emit_row "$cdocid" "$cline" "$cfull"
                        fi
                        ;;
                esac
            done < "$cfile"
        }
        scan_unit_rows() { # FILE PATTERN FULL — | WNN | rows in the inventory
            local ufile="$1" upat="$2" ufull="$3" uline uid
            while IFS= read -r uline || [ -n "$uline" ]; do
                [[ $uline =~ ^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\| ]] || continue
                [ -z "$upat" ] || [[ $uline == *"$upat"* ]] || continue
                uid="$(plan_table_cell "$uline" 2)"
                emit_row "unit:$uid" "$uline" "$ufull"
            done < "$ufile"
        }
        scan_file() {
            local docid="$1" file="$2" maxlen="$3"
            [ -f "$file" ] || return 0
            [ "$maxlen" = full ] && maxlen=0
            awk -v docid="$docid" -v pattern="$pattern" -v maxlen="$maxlen" '
                $0 ~ /^§ [0-9]+\.[0-9]+[[:space:]]*$/ { last = $2; next }
                index($0, pattern) {
                    line = $0
                    sub(/^[[:space:]]*/, "", line)
                    if (maxlen > 0 && length(line) > maxlen) line = substr(line, 1, maxlen) "..."
                    print docid "\t" (last ? last : "-") "\t" line
                }
            ' "$file"
        }
        if [ -n "$document" ]; then
            [ "$scope" = all ] || plan_die "--document and --in are mutually exclusive"
            # Resolve the document to a file; scoping to one document answers
            # "is this wording present at the surface the finding named?" —
            # plan-wide probes answer a weaker question.
            doc_file="$(plan_document_path "$plan_dir" "$document" 2>/dev/null)" || plan_die "unknown document id: $document"
            [ -f "$doc_file" ] || plan_die "document not found: $doc_file"
            matches_file="$(mktemp "${TMPDIR:-/tmp}/plan-find.XXXXXX")"
            trap 'rm -f "$matches_file"' EXIT
            case "$document" in
                coverage) scan_coverage_rows "$doc_file" coverage "$pattern" "$full" >> "$matches_file" ;;

                *) scan_file "$document" "$doc_file" "$([ "$full" = true ] && echo full || echo 120)" >> "$matches_file" ;;
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
            # BSD wc pads its count to a fixed width, so strip the padding before it
            # reaches a user-visible message.
            match_count="$(wc -l < "$matches_file" | tr -d ' ')"
            rm -f "$matches_file"
            trap - EXIT
            if [ "$match_count" -eq 0 ]; then
                printf 'plan-content.sh: find: no matches for %s (document: %s)\n' "$pattern" "$document" >&2
                exit 1
            fi
            if [ "$match_count" -gt 1 ]; then
                printf 'plan-content.sh: find: %s matches for %s (document: %s); narrow the pattern to get a single hit\n' "$match_count" "$pattern" "$document" >&2
                exit 1
            fi
            exit 0
        fi
        matches_file="$(mktemp "${TMPDIR:-/tmp}/plan-find.XXXXXX")"
        trap 'rm -f "$matches_file"' EXIT
        case "$scope" in
            plan|all) scan_file 'plan' "$plan_dir/plan-description.md" "$([ "$full" = true ] && echo full || echo 120)" >> "$matches_file" ;;
        esac
        case "$scope" in
            review|all) scan_file 'review' "$plan_dir/adversarial-review.md" "$([ "$full" = true ] && echo full || echo 120)" >> "$matches_file" ;;
        esac
        case "$scope" in
            goals|all)
                for goal_file in "$plan_dir"/*/goal.md; do
                    [ -f "$goal_file" ] || continue
                    scan_file "goal:$(basename "$(dirname "$goal_file")")" "$goal_file" "$([ "$full" = true ] && echo full || echo 120)" >> "$matches_file"
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
                    scan_file "step:$goal_name/$step_name" "$step_file" "$([ "$full" = true ] && echo full || echo 120)" >> "$matches_file"
                done
                ;;
        esac
        case "$scope" in
            testing|all)
                for step_file in "$plan_dir"/*/steps/*-testing.md; do
                    [ -f "$step_file" ] || continue
                    goal_name="$(basename "$(dirname "$(dirname "$step_file")")")"
                    step_name="$(basename "$step_file" .md)"
                    scan_file "step:$goal_name/$step_name" "$step_file" "$([ "$full" = true ] && echo full || echo 120)" >> "$matches_file"
                done
                ;;
        esac
        case "$scope" in
            units|inventory|all)
                [ -f "$plan_dir/work-unit-inventory.md" ] \
                    && scan_unit_rows "$plan_dir/work-unit-inventory.md" "$pattern" "$full" >> "$matches_file"
                ;;
        esac
        case "$scope" in
            coverage|all)
                [ -f "$plan_dir/work-unit-inventory.md" ] \
                    && scan_coverage_rows "$plan_dir/work-unit-inventory.md" coverage "$pattern" "$full" >> "$matches_file"
                ;;
        esac
        case "$scope" in
            stories|all)
                [ -f "$plan_dir/ui-user-stories.md" ] && awk -v docid="stories" -v pattern="$pattern" -v full="$full" '
                    /^## / { section = $0 }
                    index($0, pattern) && $0 !~ /^# / {
                        line = $0; sub(/^[[:space:]]*/, "", line)
                        if (full != "true" && length(line) > 120) line = substr(line, 1, 120) "..."
                        print docid "\t" (section ? section : "-") "\t" line
                    }
                ' "$plan_dir/ui-user-stories.md" >> "$matches_file"
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
        # BSD wc pads its count to a fixed width, so strip the padding before it
        # reaches a user-visible message.
        match_count="$(wc -l < "$matches_file" | tr -d ' ')"
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
    diff)
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage
        # The diff subcommand lives in a sibling library so this file stays
        # under the CODE-STYLE.md §3 size limit.
        source "$script_dir/plan-content-diff-lib.sh"
        plan_content_diff "$@"
        ;;
    *) usage ;;
esac
