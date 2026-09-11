#!/usr/bin/env bash
# MODE: PROD
# plan-content-lib.sh — the plan-content.sh find subcommand (CODE-STYLE §3,
# 400-line script cap).
#
# Sourced by plan-content.sh only, after plan-document-lib.sh. content_find_command
# reads plan_die/plan_document_path/plan_table_cell/plan_require_directory from
# the caller's sourced libraries, and takes the same positional arguments the
# find subcommand does.

set -euo pipefail
export LC_ALL=C

content_find_command() { # <plan-dir> <pattern> [--in SCOPE] [--document ID] [--full] [--format text|json]
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
}
