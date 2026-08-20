#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# GENERATED FILE — do not edit. Compiled from scripts/lib/document/*.sh by:
#   planning/scripts/build-plan-libs.sh
# Edit the function file in that directory, then re-run the build.
# Target: prod
#
# sections, paragraphs, titles and fields

set -euo pipefail

[ -z "${PLAN_DOCUMENT_LIB_LOADED:-}" ] || return 0
PLAN_DOCUMENT_LIB_LOADED=1

# The façade every planning script sources. It pulls in the sibling libraries so
# that `source plan-document-lib.sh` provides the same symbols it always did:
# 40-plus scripts source this path, and the split must be invisible to them.
#
# Sorted last in the group (99-) so every definition above it exists before the
# initialisation block runs.
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-core-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-table-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-progress-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-map-lib.sh"
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-inventory-lib.sh"

# ── Load-time initialisation ─────────────────────────────────────────────────
# Guarded: this library is sourced more than once per process, and re-running it
# would reset plan_error_count and record plan_cleanup as its own "prior" handler.
if [ -z "${PLAN_DOCUMENT_LIB_INITIALISED:-}" ]; then
    PLAN_DOCUMENT_LIB_INITIALISED=1
    plan_error_count=0
    plan_tmp_files=()
    plan_prior_exit_trap="$(trap -p EXIT)"
    trap plan_cleanup EXIT INT TERM
fi

# Delete one numbered paragraph and renumber the rest of its section so labels
# stay sequential. Targeted rather than re-emitting the section, which risks a
# transcription slip damaging paragraphs no finding was about.
plan_delete_paragraph() {
    local file="$1" paragraph_id="$2" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
    trap 'rm -f "$temporary_file"' RETURN
    awk -v wanted="$paragraph_id" '
        BEGIN {
            target_value = wanted
            sub(/^§ /, "", target_value)
            split(target_value, target_parts, /\./)
            target_section = target_parts[1]
            target_number = target_parts[2] + 0
        }
        /^§ [0-9]+\.[0-9]+$/ {
            current_value = $0
            sub(/^§ /, "", current_value)
            split(current_value, current_parts, /\./)
            section = current_parts[1]
            number = current_parts[2] + 0
            if (section == target_section && number == target_number) {
                if (target_found++) exit 2
                skipping = 1
                next
            }
            skipping = 0
            if (section == target_section && number > target_number) {
                print "§ " section "." (number - 1)
            } else {
                print
            }
            next
        }
        /^## / {
            # A section boundary always stops the delete: never swallow a
            # following heading even when the deleted paragraph was the last in
            # its section (that would re-parent the next section under it).
            skipping = 0
            print
            next
        }
        skipping { next }
        { print }
        END { if (target_found != 1) exit 3 }
    ' "$file" > "$temporary_file" || plan_die "Paragraph was not found exactly once: $paragraph_id"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_document_kind() {
    case "$1" in
        plan) printf '%s\n' plan ;;
        adversarial-review) printf '%s\n' review ;;
        coverage|inventory|stories|bugs|fixes|fix-keys|fixkeys|approval|progress) printf '%s\n' reference ;;
        goal-progress:*) printf '%s\n' reference ;;
        goal:*) printf '%s\n' goal ;;
        step:*)
            # A step id ending in -testing names the step's testing companion,
            # which has its own writable sections (Automated tests, ...).
            case "$1" in
                *-testing) printf '%s\n' testing ;;
                *) printf '%s\n' step ;;
            esac
            ;;
        unit:*) printf '%s\n' step ;;
        *) plan_die "Unknown document ID: $1" ;;
    esac
}

plan_document_path() {
    local plan_dir="$1" document_id="$2" unit goal step
    case "$document_id" in
        plan)
            printf '%s\n' "$plan_dir/plan-description.md"
            ;;
        adversarial-review)
            printf '%s\n' "$plan_dir/adversarial-review.md"
            ;;
        coverage|inventory)
            printf '%s\n' "$plan_dir/work-unit-inventory.md"
            ;;
        progress)
            printf '%s\n' "$plan_dir/progress.md"
            ;;
        goal-progress:*)
            goal="${document_id#goal-progress:}"
            [ -n "$goal" ] || plan_die "Goal progress IDs use goal-progress:<goal>"
            printf '%s\n' "$plan_dir/$goal/progress.md"
            ;;
        stories)
            printf '%s\n' "$plan_dir/ui-user-stories.md"
            ;;
        bugs)
            printf '%s\n' "$plan_dir/bugs.md"
            ;;
        fixes)
            printf '%s\n' "$plan_dir/fixes.md"
            ;;
        fix-keys|fixkeys)
            printf '%s\n' "$plan_dir/fix-keys.json"
            ;;
        approval)
            printf '%s\n' "$plan_dir/approval.json"
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
            # A trailing -testing names the step's testing companion, which is
            # a writable surface of its own (executors run the procedure it
            # records). It resolves to steps/<step>-testing.md.
            printf '%s\n' "$plan_dir/$goal/steps/$step.md"
            ;;
        unit:W*)
            unit="${document_id#unit:}"
            if plan_inventory_row "$plan_dir/work-unit-inventory.md" "$unit"; then
                goal="$plan_inventory_goal"
                step="$plan_inventory_step"
            fi
            [ -n "${goal:-}" ] && [ -n "${step:-}" ] || plan_die "Work unit not found: $unit"
            printf '%s\n' "$plan_dir/$goal/steps/$step.md"
            ;;
        *)
            plan_die "Unknown document ID: $document_id (use plan, adversarial-review, goal:<goal>, goal-progress:<goal>, step:<goal>/<step>, unit:<WNN>, coverage, inventory, progress, stories, bugs, fixes, fix-keys, or approval)"
            ;;
    esac
}

plan_insert_paragraph() {
    local file="$1" paragraph_id="$2" mode="$3" body_file="$4" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    case "$mode" in before|after) ;; *) plan_die "Paragraph insertion mode must be before or after" ;; esac
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
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

# A section form can only rewrite a section the document already holds, so the
# refusal has to say which sections it does hold. Without that, a valid section
# id that this particular file never received reads as a broken helper: a
# reviewer hit `-ss ... browser-verification` on a companion created without
# browser content, was told only "not found exactly once", and inferred a remedy
# (re-create with --overwrite) that cannot work -- create-step-testing.sh emits
# `## Automated tests` and nothing else.
plan_missing_section_message() {
    local file="$1" heading="$2" present
    present="$(grep '^## ' "$file" 2>/dev/null | tr '\n' ' ')"
    printf '%s not found in %s' "$heading" "${file##*/}"
    if [ -n "$present" ]; then
        printf '; it has: %s' "$present"
    fi
    printf -- ' -- a section form rewrites a section that already exists, it cannot add one. '
    case "$file" in
        *-testing.md) printf 'A testing companion carries only the sections its creator emitted; create-step-testing.sh emits "## Automated tests".' ;;
        *) printf 'Create the document with the helper that owns it, then rewrite the section.' ;;
    esac
}

# A section holding `- Label:` lines is field-shaped whatever the allow-list says,
# and rewriting it removes labels another mechanism may own -- `- Status:` in
# `## Verdict` belongs to the review-status gate. Refuse rather than destroy.
plan_refuse_field_section() {
    local file="$1" heading="$2" shape
    [ -f "$file" ] || return 0
    # A section whose body carries `- Label:` lines is field-shaped, and one
    # whose body OPENS with a table row is table-shaped. A narrative section may
    # still contain a table paragraph, which is why the discriminator is the
    # first body line rather than the presence of a pipe anywhere.
    shape="$(awk -v want="$heading" '
        $0 == want { inside = 1; next }
        inside && /^## / { exit }
        inside && /^[[:space:]]*$/ { next }
        inside && /^- [A-Z][^:]*:/ { fields++ }
        inside && first == "" { first = ($0 ~ /^\|/) ? "table" : "other" }
        END {
            if (fields > 0) print "fields"
            else if (first == "table") print "table"
            else print "narrative"
        }' "$file")"
    case "$shape" in
        fields)
            plan_die "Section '$heading' holds fields (- Label: value); rewriting it would remove them, and a field there may belong to another gate. Write one field at a time with --field." 65 ;;
        table)
            plan_die "Section '$heading' is a table; rewriting it as paragraphs would discard every row. Use the helper that owns that table." 65 ;;
    esac
}

plan_render_paragraphs() {
    local number="$1" content="$2"
    [ -n "$content" ] || plan_die "Section content must not be empty"
    printf '%s' "$content" | awk -v number="$number" '
        BEGIN { RS=""; ORS="" }
        {
            text = $0
            # No "\n" inside the bracket expression: a backslash escape there
            # is undefined in POSIX awk, and [[:space:]] already covers newline.
            sub(/^[[:space:]]+/, "", text)
            sub(/[[:space:]]+$/, "", text)
            if (text == "") next
            if (count++) printf "\n\n"
            printf "§ %s.%d\n%s", number, count, text
        }
        END { if (count == 0) exit 1 }
    '
}

plan_replace_field() {
    local file="$1" label="$2" value="$3" temporary_file
    plan_require_safe_value "$label" "$value"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
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

plan_replace_paragraph() {
    local file="$1" paragraph_id="$2" content="$3" temporary_file
    [[ "$paragraph_id" =~ ^§[[:space:]][0-9]+\.[0-9]+$ ]] || plan_die "Paragraph ID must use the form '§ 2.1'"
    [[ "$content" != *$'\n\n'* ]] || plan_die "A paragraph replacement must contain exactly one paragraph; use section for multiple paragraphs"
    [ -n "$content" ] || plan_die "Paragraph content must not be empty"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
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

plan_replace_section() {
    local file="$1" heading="$2" body_file="$3" temporary_file
    plan_refuse_field_section "$file" "$heading"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
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
    ' "$file" > "$temporary_file" || plan_die "$(plan_missing_section_message "$file" "$heading")"
    mv "$temporary_file" "$file"
    trap - RETURN
}

plan_replace_title() {
    local file="$1" title="$2" temporary_file
    plan_require_safe_value title "$title"
    [[ "$title" != *$'\n'* ]] || plan_die "Title must be one line"
    temporary_file="${file}.tmp.$$"
    plan_register_temp_file "$temporary_file"
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
        plan/environment-facts) printf '%s\t%s\n' '## Environment facts' 9 ;;
        plan/approach-decisions) printf '%s\t%s\n' '## Approach decisions' 10 ;;
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
        testing/automated-tests) printf '%s\t%s\n' '## Automated tests' 2 ;;
        testing/browser-verification) printf '%s\t%s\n' '## Browser verification' 3 ;;
        testing/backend-verification) printf '%s\t%s\n' '## Backend verification' 4 ;;
        testing/manual-verification) printf '%s\t%s\n' '## Manual verification' 5 ;;
        review/review-scope) printf '%s\t%s\n' '## Review scope' 1 ;;
        review/findings) printf '%s\t%s\n' '## Findings' 2 ;;
        review/rationale) printf '%s\t%s\n' '## Verdict' 3 ;;
        *) plan_die "$(plan_unknown_section "$kind" "$section")" ;;
    esac
}

# Concise, agent-friendly error for an unknown narrative section: list the
# valid ids for the document kind and, when one is close, suggest it.
plan_unknown_section() {
    local kind="$1" section="$2" valid id close="" best=""
    case "$kind" in
        plan) valid="current-state desired-outcome approach approach-decisions scope affected-areas constraints-and-decisions risks-and-open-questions environment-facts" ;;
        goal) valid="current-state-and-prior-goal-handoffs outcome-and-definition-of-done why-this-goal-is-needed scope affected-areas dependencies-and-handoffs implementation-approach-risks-and-edge-cases owned-work-units goal-size-exception" ;;
        step) valid="objective instructions acceptance-criteria handoff" ;;
        testing) valid="automated-tests browser-verification backend-verification manual-verification" ;;
        # A review has no narrative section: Review scope and Verdict hold
        # fields (-f writes them, one label at a time) and Findings is a table
        # (update-adversarial-review.sh writes it). A section rewrite here
        # dropped `- Status:` and left the plan unapprovable.
        review) valid="" ;;
        *) valid="" ;;
    esac
    for id in $valid; do
        if [ "$id" = "$section" ]; then
            printf 'Section is valid: %s\n' "$section"
            return 0
        fi
        # Simple closeness: same prefix or >50% shared prefix length.
        if [[ "$id" == "$section"* ]] || [[ "$section" == "$id"* ]]; then
            [ -z "$close" ] && close="$id"
        fi
        if [ -z "$best" ] || [ "${#id}" -lt "${#best}" ]; then
            # crude nearest: shortest id differing in fewest chars by prefix
            :
        fi
    done
    printf "Section '%s' is not a mutable narrative section for a %s document.\n" "$section" "$kind"
    if [ -n "$close" ]; then
        printf 'Closest match: %s\n' "$close"
    fi
    printf 'Valid %s section ids: %s\n' "$kind" "$valid"
}
