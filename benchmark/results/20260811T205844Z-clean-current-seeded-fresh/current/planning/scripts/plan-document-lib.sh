#!/usr/bin/env bash
# Shared helpers for the planning document commands. This file is sourced.

set -euo pipefail

plan_default_root() {
    if [ -n "${PLANS_ROOT:-}" ]; then
        printf '%s\n' "${PLANS_ROOT%/}"
        return 0
    fi
    local home_dir="${HOME:-}"
    if [ -z "$home_dir" ] && [ -n "${USERPROFILE:-}" ]; then
        home_dir="$USERPROFILE"
    fi
    if [ -z "$home_dir" ] && [ -n "${HOMEDRIVE:-}${HOMEPATH:-}" ]; then
        home_dir="${HOMEDRIVE:-}${HOMEPATH:-}"
    fi
    [ -n "$home_dir" ] || plan_die "Unable to resolve the user home directory; set PLANS_ROOT"
    printf '%s/.plans\n' "${home_dir%/}"
}

plan_ensure_root_permissions() {
    local root="${1:-$(plan_default_root)}" helper_dir="${2:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)}"
    local probe
    mkdir -p "$root" || plan_die "Cannot create plan root: $root"
    [ -d "$root" ] && [ -r "$root" ] && [ -w "$root" ] && [ -x "$root" ] \
        || plan_die "Plan root is not readable, writable, and searchable: $root"
    probe="$root/.permission-probe.$$"
    ( : > "$probe" && rm -f "$probe" ) || plan_die "Plan root does not permit file editing: $root"
    [ -d "$helper_dir" ] && [ -r "$helper_dir" ] && [ -x "$helper_dir" ] \
        || plan_die "Planning helper directory is not readable/searchable: $helper_dir"
    find "$helper_dir" -maxdepth 1 -type f -name '*.sh' -exec test -r {} \; -exec test -x {} \; \
        || plan_die "One or more planning helpers cannot be read and executed: $helper_dir"
    printf '%s\n' "$root"
}

plan_die() {
    printf '%s\n' "$*" >&2
    exit 64
}

plan_require_directory() {
    [ -d "$1" ] || plan_die "Plan directory not found: $1"
}

plan_require_safe_value() {
    local label="$1" value="$2"
    [ -n "$value" ] || plan_die "$label must not be empty"
    [[ "$value" != *'|'* ]] || plan_die "$label must not contain a Markdown table separator (|)"
    [[ "$value" != *$'\n'* && "$value" != *$'\r'* ]] || plan_die "$label must be one line"
}

plan_decode_escaped_newlines() {
    local value="$1"
    printf '%s' "${value//\\n/$'\n'}"
}

plan_document_path() {
    local plan_dir="$1" document_id="$2" unit goal step
    case "$document_id" in
        plan)
            printf '%s\n' "$plan_dir/plan-description.md"
            ;;
        review)
            printf '%s\n' "$plan_dir/adversarial-review.md"
            ;;
        goal:*)
            goal="${document_id#goal:}"
            printf '%s\n' "$plan_dir/$goal/goal.md"
            ;;
        step:*)
            goal="${document_id#step:}"
            step="${goal#*/}"
            goal="${goal%%/*}"
            [ -n "$step" ] && [ "$step" != "$goal" ] || plan_die "Step document IDs use step:<goal>/<step>"
            printf '%s\n' "$plan_dir/$goal/steps/$step.md"
            ;;
        unit:W*)
            unit="${document_id#unit:}"
            IFS=$'\t' read -r goal step < <(
                awk -F'|' -v wanted="$unit" '
                    function trim(value) { gsub(/^[[:space:]]+|[[:space:]]+$/, "", value); return value }
                    /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ && trim($2) == wanted {
                        print trim($9) "\t" trim($10)
                    }
                ' "$plan_dir/work-unit-inventory.md"
            )
            [ -n "${goal:-}" ] && [ -n "${step:-}" ] || plan_die "Work unit not found: $unit"
            printf '%s\n' "$plan_dir/$goal/steps/$step.md"
            ;;
        *)
            plan_die "Unknown document ID: $document_id (use plan, review, goal:<goal>, step:<goal>/<step>, or unit:<WNN>)"
            ;;
    esac
}

plan_document_kind() {
    case "$1" in
        plan) printf '%s\n' plan ;;
        review) printf '%s\n' review ;;
        goal:*) printf '%s\n' goal ;;
        step:*|unit:*) printf '%s\n' step ;;
        *) plan_die "Unknown document ID: $1" ;;
    esac
}

# Print the required heading and its paragraph-number prefix for a mutable
# narrative section. Structured sections are intentionally excluded.
plan_section_spec() {
    local kind="$1" section="$2"
    case "$kind/$section" in
        plan/current-state) printf '%s\t%s\n' '## Current state' 2 ;;
        plan/desired-outcome) printf '%s\t%s\n' '## Desired outcome' 3 ;;
        plan/approach) printf '%s\t%s\n' '## Approach' 4 ;;
        plan/scope) printf '%s\t%s\n' '## Scope' 5 ;;
        plan/affected-areas) printf '%s\t%s\n' '## Affected areas' 6 ;;
        plan/constraints-and-decisions) printf '%s\t%s\n' '## Constraints and decisions' 7 ;;
        plan/risks-and-open-questions) printf '%s\t%s\n' '## Risks and open questions' 8 ;;
        goal/current-state-and-prior-goal-handoffs) printf '%s\t%s\n' '## Current state and prior-goal handoffs' 2 ;;
        goal/outcome-and-definition-of-done) printf '%s\t%s\n' '## Outcome and definition of done' 3 ;;
        goal/why-this-goal-is-needed) printf '%s\t%s\n' '## Why this goal is needed' 4 ;;
        goal/scope) printf '%s\t%s\n' '## Scope' 5 ;;
        goal/affected-areas) printf '%s\t%s\n' '## Affected files, systems, data, and interfaces' 6 ;;
        goal/dependencies-and-handoffs) printf '%s\t%s\n' '## Dependencies and handoffs' 7 ;;
        goal/implementation-approach-risks-and-edge-cases) printf '%s\t%s\n' '## Implementation approach, risks, and edge cases' 8 ;;
        goal/owned-work-units) printf '%s\t%s\n' '## Owned work units' 9 ;;
        goal/goal-size-exception) printf '%s\t%s\n' '## Goal-size exception' 11 ;;
        step/objective) printf '%s\t%s\n' '## Objective' 4 ;;
        step/instructions) printf '%s\t%s\n' '## Instructions' 5 ;;
        step/acceptance-criteria) printf '%s\t%s\n' '## Acceptance criteria' 6 ;;
        step/handoff) printf '%s\t%s\n' '## Handoff' 7 ;;
        review/review-scope) printf '%s\t%s\n' '## Review scope' 1 ;;
        review/findings) printf '%s\t%s\n' '## Findings' 2 ;;
        review/rationale) printf '%s\t%s\n' '## Verdict' 3 ;;
        *) plan_die "Section '$section' is not a mutable narrative section for a $kind document" ;;
    esac
}

plan_replace_testing_requirement() {
    local file="$1" required="$2" rationale="$3" replacement temporary_file
    case "$required" in
        yes|no) ;;
        *) plan_die "Test requirement must be yes or no" ;;
    esac
    plan_require_safe_value rationale "$rationale"
    replacement="| $required | $rationale |"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v replacement="$replacement" '
        $0 == "## Testing requirement" {
            in_section = 1
            print
            next
        }
        in_section && /^## / {
            in_section = 0
        }
        in_section && $0 == "| Test required | Rationale |" {
            header = 1
            print
            next
        }
        in_section && header && /^\|---\|---\|$/ {
            separator = 1
            print
            next
        }
        in_section && separator && /^\|[^|]+\|[^|]+\|$/ {
            if (data_row++) exit 3
            print replacement
            next
        }
        { print }
        END {
            if (!header || !separator || data_row != 1) exit 2
        }
    ' "$file" > "$temporary_file" || plan_die "Testing requirement table was not found exactly once: $file"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_testing_requirement_for_goal() {
    local goal_file="$1"
    awk -F'|' '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { exit }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|/ {
            value = $2
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
            print value
            exit
        }
    ' "$goal_file"
}

plan_emit_step_testing_reminder() {
    local plan_dir="$1" document_id="$2" step_file goal_dir goal_file required companion
    case "$document_id" in
        step:*|unit:*) ;;
        *) return 0 ;;
    esac
    step_file="$(plan_document_path "$plan_dir" "$document_id")"
    goal_dir="$(dirname "$(dirname "$step_file")")"
    goal_file="$goal_dir/goal.md"
    [ -f "$goal_file" ] || return 0
    required="$(plan_testing_requirement_for_goal "$goal_file")"
    [ "$required" = yes ] || return 0
    companion="${step_file%.md}-testing.md"
    if [ -f "$companion" ]; then
        printf 'Reminder: testing instructions already exist at %s; review them for accuracy and completeness after updating this step.\n' "$companion"
    else
        printf 'Reminder: this goal requires testing; continue with its test/proof step before marking the goal complete.\n'
    fi
}

plan_render_paragraphs() {
    local number="$1" content="$2"
    [ -n "$content" ] || plan_die "Section content must not be empty"
    printf '%s' "$content" | awk -v number="$number" '
        BEGIN { RS=""; ORS="" }
        {
            text = $0
            sub(/^[[:space:]\n]+/, "", text)
            sub(/[[:space:]\n]+$/, "", text)
            if (text == "") next
            if (count++) printf "\n\n"
            printf "§ %s.%d\n%s", number, count, text
        }
        END { if (count == 0) exit 1 }
    '
}

plan_replace_section() {
    local file="$1" heading="$2" body_file="$3" temporary_file
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v heading="$heading" -v replacement="$body_file" '
        BEGIN {
            while ((getline line < replacement) > 0) {
                body = body (body == "" ? "" : "\n") line
            }
            close(replacement)
        }
        $0 == heading {
            if (found++) exit 2
            print
            print ""
            print body
            skipping = 1
            next
        }
        skipping && /^## / { skipping = 0; print "" }
        !skipping { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Section heading was not found exactly once: $heading"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_replace_paragraph() {
    local file="$1" paragraph_id="$2" content="$3" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    [[ "$content" != *$'\n\n'* ]] || plan_die "A paragraph replacement must contain exactly one paragraph; use section for multiple paragraphs"
    [ -n "$content" ] || plan_die "Paragraph content must not be empty"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" -v replacement="$content" '
        $0 == wanted {
            if (found++) exit 2
            print
            print replacement
            skipping = 1
            next
        }
        skipping && ($0 == "" || /^§[[:space:]][0-9]+\.[0-9]+$/ || /^## /) {
            if ($0 ~ /^§[[:space:]][0-9]+\.[0-9]+$/ || $0 ~ /^## /) print ""
            skipping = 0
        }
        !skipping { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_render_csv_table() {
    local columns="$1" csv="$2" csv_file
    [[ "$columns" =~ ^[1-9][0-9]*$ ]] || plan_die "Table column count must be a positive integer"
    csv_file="$(mktemp "${TMPDIR:-/tmp}/plan-table.XXXXXX")"
    trap 'rm -f "$csv_file"' RETURN
    plan_decode_escaped_newlines "$csv" > "$csv_file"
    awk -v expected="$columns" '
        function parse_csv(line, fields,    i, ch, next_ch, quoted, field, count) {
            for (i = 1; i <= length(line); i++) {
                ch = substr(line, i, 1)
                if (ch == "\\" && substr(line, i + 1, 1) == "\"") {
                    field = field "\""
                    i++
                } else if (ch == "\"") {
                    next_ch = substr(line, i + 1, 1)
                    if (quoted && next_ch == "\"") {
                        field = field "\""
                        i++
                    } else {
                        quoted = !quoted
                    }
                } else if (ch == "," && !quoted) {
                    fields[++count] = field
                    field = ""
                } else {
                    field = field ch
                }
            }
            if (quoted) return -1
            fields[++count] = field
            return count
        }
        function emit_row(fields, count,    i) {
            printf "|"
            for (i = 1; i <= count; i++) {
                if (fields[i] ~ /\|/ || fields[i] ~ /\r/) exit 4
                printf " %s |", fields[i]
            }
            printf "\n"
        }
        {
            if ($0 ~ /^[[:space:]]*$/) exit 5
            count = parse_csv($0, fields)
            if (count < 0) exit 2
            if (count != expected) exit 3
            emit_row(fields, count)
            if (NR == 1) {
                printf "|"
                for (i = 1; i <= expected; i++) printf "---|"
                printf "\n"
            }
        }
        END { if (NR == 0) exit 6 }
    ' "$csv_file" || plan_die "CSV table must have $columns columns on every non-empty row and no pipe characters"
    rm -f "$csv_file"
    trap - RETURN
}

plan_insert_paragraph() {
    local file="$1" paragraph_id="$2" mode="$3" body_file="$4" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    case "$mode" in before|after) ;; *) plan_die "Paragraph insertion mode must be before or after" ;; esac
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" -v mode="$mode" -v body_file="$body_file" '
        function output(line) {
            print line
            previous_blank = (line == "")
        }
        function emit_insertion(    count, i, lines) {
            if (!previous_blank) output("")
            output("§ " target_section "." insertion_number)
            count = split(body, lines, "\n")
            for (i = 1; i <= count; i++) output(lines[i])
            output("")
        }
        BEGIN {
            while ((getline line < body_file) > 0) body = body (body == "" ? "" : "\n") line
            close(body_file)
            target_value = wanted
            sub(/^§ /, "", target_value)
            split(target_value, target_parts, /\./)
            target_section = target_parts[1]
            target_number = target_parts[2] + 0
            insertion_number = (mode == "after" ? target_number + 1 : target_number)
        }
        {
            line = $0
            is_paragraph = (line ~ /^§ [0-9]+\.[0-9]+$/)
            if (is_paragraph) {
                current_value = line
                sub(/^§ /, "", current_value)
                split(current_value, current_parts, /\./)
                section = current_parts[1]
                number = current_parts[2] + 0
                is_target = (section == target_section && number == target_number)
                if (is_target && target_found++) exit 2
                if (is_target && mode == "before") emit_insertion()
                if (pending_after && (is_paragraph || line ~ /^## /)) {
                    emit_insertion()
                    pending_after = 0
                }
                if (is_target && mode == "after") pending_after = 1
                if (section == target_section && number >= insertion_number) {
                    line = "§ " section "." (number + 1)
                }
            } else if (pending_after && line ~ /^## /) {
                emit_insertion()
                pending_after = 0
            }
            output(line)
        }
        END {
            if (pending_after) emit_insertion()
            if (target_found != 1) exit 3
        }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_replace_field() {
    local file="$1" label="$2" value="$3" temporary_file
    plan_require_safe_value "$label" "$value"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v label="$label" -v replacement="$value" '
        $0 ~ "^- " label ":" {
            if (found++) exit 2
            print "- " label ": " replacement
            next
        }
        { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Field was not found exactly once: $label"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_replace_title() {
    local file="$1" title="$2" temporary_file
    plan_require_safe_value title "$title"
    [[ "$title" != *$'\n'* ]] || plan_die "Title must be one line"
    temporary_file="${file}.tmp.$$"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v replacement="$title" '
        /^# / {
            if (found++) exit 2
            sub(/:.*/, ": " replacement)
            print
            next
        }
        { print }
        END { if (found != 1) exit 2 }
    ' "$file" > "$temporary_file" || plan_die "Document title was not found exactly once"
    mv "$temporary_file" "$file"
    trap - RETURN
}
