#!/usr/bin/env bash
# Shared helpers for the planning document commands. This file is sourced.

set -euo pipefail

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
        goal/goal-size-exception) printf '%s\t%s\n' '## Goal-size exception' 10 ;;
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
