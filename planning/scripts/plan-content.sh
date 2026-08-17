#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat >&2 <<'USAGE'
Usage:
  plan-content.sh get <plan-directory> <document-id> [markdown|text|json|path]
  plan-content.sh summary <plan-directory> [markdown|text|json]
  plan-content.sh blast-radius <plan-directory> <WNN|goal-name|goal-name/step-name> [markdown|text|json]
  plan-content.sh find <plan-directory> <pattern> [--in plan|goals|steps|units|review|testing|coverage|stories|all] [--document <docid>] [--full] [--format text|json]
                                    literal search; prints docid<TAB>section<TAB>excerpt per match,
                                    exits 1 on zero or multiple matches; --document scopes to one document
                                    (plan, review, coverage, stories, goal:<g>, step:<g>/<s>, unit:<WNN>, or a
                                    step:-testing id); --full disables excerpt truncation
  plan-content.sh diff <plan-directory> <git-ref> [--format text|json]
                                    lists documents changed since git-ref and the
                                    paragraph labels touched in each
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
        case "$scope" in plan|goals|steps|units|review|testing|coverage|stories|all) ;; *) plan_die "Unknown scope: $scope (use plan, goals, steps, units, review, testing, coverage, stories, or all)" ;; esac
        case "$format" in text|json) ;; *) plan_die "Unknown format: $format (use text or json)" ;; esac
        # Literal scan of one document (used by both --document and the scoped
        # branches below; defined here so --document can call it).
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
                coverage) awk -F'|' -v pattern="$pattern" -v full="$full" '
                        /^## Definition-of-done coverage/ { in_coverage = 1; next }
                        in_coverage && /^## / { exit }
                        in_coverage && /^\|/ && index($0, pattern) {
                            row = $0; gsub(/^[[:space:]]*\|[[:space:]]*/, "", row); gsub(/[[:space:]]*\|[[:space:]]*$/, "", row)
                            if (full != "true" && length(row) > 120) row = substr(row, 1, 120) "..."
                            print "coverage\t" row "\t" row
                        }
                    ' "$doc_file" >> "$matches_file"
                    ;;
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
            match_count="$(wc -l < "$matches_file")"
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
            units|all)
                [ -f "$plan_dir/work-unit-inventory.md" ] && awk -F'|' -v pattern="$pattern" -v full="$full" '
                    /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
                        id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
                        if (index($0, pattern)) {
                            row = $0; gsub(/^[[:space:]]*\|[[:space:]]*/, "", row); gsub(/[[:space:]]*\|[[:space:]]*$/, "", row)
                            if (full != "true" && length(row) > 120) row = substr(row, 1, 120) "..."
                            print "unit:" id "\t" row "\t" row
                        }
                    }
                ' "$plan_dir/work-unit-inventory.md" >> "$matches_file"
                ;;
        esac
        case "$scope" in
            coverage|all)
                [ -f "$plan_dir/work-unit-inventory.md" ] && awk -F'|' -v pattern="$pattern" -v full="$full" '
                    /^## Definition-of-done coverage/ { in_coverage = 1; next }
                    in_coverage && /^## / { exit }
                    in_coverage && /^\|/ && index($0, pattern) {
                        row = $0; gsub(/^[[:space:]]*\|[[:space:]]*/, "", row); gsub(/[[:space:]]*\|[[:space:]]*$/, "", row)
                        if (full != "true" && length(row) > 120) row = substr(row, 1, 120) "..."
                        print "coverage\t" row "\t" row
                    }
                ' "$plan_dir/work-unit-inventory.md" >> "$matches_file"
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
    diff)
        [ "$#" -ge 2 ] && [ "$#" -le 3 ] || usage
        plan_dir="$1"; git_ref="$2"; format="${3:-text}"
        plan_require_directory "$plan_dir"
        case "$format" in text|json) ;; *) plan_die "Unknown format: $format (use text or json)" ;; esac
        # The plan may be a subdirectory of a repo that covers it (e.g. a
        # git-excluded .plans root or an initiative repo holding sibling plans),
        # so walk up to the enclosing repo and scope the diff to the plan dir.
        repo_root="$(git -C "$plan_dir" rev-parse --show-toplevel 2>/dev/null || true)"
        if [ -z "$repo_root" ]; then
            plan_die "plan directory is not inside a git repository: $plan_dir"
        fi
        repo_root="$(cd "$repo_root" && pwd -P)"
        plan_abs="$(cd "$plan_dir" && pwd -P)"
        plan_rel="."
        if [ "$plan_abs" != "$repo_root" ]; then
            plan_rel="${plan_abs#"$repo_root"/}"
        fi
        # name-only, scoped to the plan subtree (relative paths).
        changed="$(git -C "$repo_root" diff --name-only "$git_ref" -- "$plan_rel" 2>/dev/null | sed "s#^$plan_rel/##" | grep -v '^$' || true)"
        if [ -z "$changed" ]; then
            git -C "$repo_root" rev-parse -q --verify "$git_ref" >/dev/null 2>&1 \
                || plan_die "git ref not found: $git_ref"
            printf 'No plan documents changed since %s\n' "$git_ref"
            exit 0
        fi
        # A changed paragraph label is usually an UNCHANGED diff line (only its
        # content changed), so map each changed line on the new-file side back
        # to the enclosing "§ N.N" label.
        python3 - "$plan_abs" "$plan_rel" "$repo_root" "$git_ref" "$format" "$changed" <<'PY'
import json, subprocess, sys

plan_abs, plan_rel, repo_root, git_ref, fmt = sys.argv[1:6]
docs = [d for d in sys.argv[6].splitlines() if d]

def diff_lines(doc):
    path = f"{plan_rel}/{doc}" if plan_rel != "." else doc
    out = subprocess.run(
        ["git", "-C", repo_root, "diff", "-U0", git_ref, "--", path],
        capture_output=True, text=True).stdout
    # Parse new-file side: after "@@ -a,b +c,d @@", new-file lines are numbered
    # from c; '+' lines are added content, context lines exist in both.
    new_line = None
    added = []
    for raw in out.splitlines():
        if raw.startswith("@@"):
            m = __import__("re").search(r"\+(\d+)", raw)
            new_line = int(m.group(1)) if m else None
            continue
        if new_line is None:
            continue
        if raw.startswith("+") and not raw.startswith("+++"):
            added.append(new_line)
        if not raw.startswith("-"):
            new_line += 1
    if not added:
        return []
    # Find the label enclosing each added new-file line.
    with open(f"{plan_abs}/{doc}", encoding="utf-8") as fh:
        lines = fh.read().splitlines()
    label_by_line = {}
    current = None
    for i, line in enumerate(lines, start=1):
        if line.startswith("§ ") and __import__("re").match(r"^§ [0-9]+\.[0-9]+$", line):
            current = line
        label_by_line[i] = current
    labels = []
    for ln in added:
        lab = label_by_line.get(ln)
        if lab and lab not in labels:
            labels.append(lab)
    return labels

if fmt == "text":
    for doc in docs:
        print(f"## {doc}")
        for label in diff_lines(doc):
            print(label)
else:
    rows = []
    for doc in docs:
        rows.append({"document": doc, "paragraphs": diff_lines(doc)})
    print(json.dumps({"ref": git_ref, "documents": rows}))
PY
        ;;
    *) usage ;;
esac
