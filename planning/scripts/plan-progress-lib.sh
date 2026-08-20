#!/usr/bin/env bash
# GENERATED FILE — do not edit. Compiled from scripts/lib/progress/*.sh by:
#   planning/scripts/build-plan-libs.sh
# Edit the function file in that directory, then re-run the build.
#
# progress arithmetic and the status glyphs

set -euo pipefail

[ -z "${PLAN_PROGRESS_LIB_LOADED:-}" ] || return 0
PLAN_PROGRESS_LIB_LOADED=1

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
        printf 'Reminder: testing instructions already exist at %s; review them for accuracy and completeness after updating this step.\n' "$companion" >&2
    else
        printf 'Reminder: this goal requires testing; continue with its test/proof step before marking the goal complete.\n' >&2
    fi
}

plan_progress_bar() {
    local completed="$1" total="$2" width="${3:-20}" percent filled empty
    percent="$(plan_progress_percent "$completed" "$total")"
    filled=$(( percent * width / 100 ))
    empty=$(( width - filled ))
    printf '%s%s\n' \
        "$(printf '%*s' "$filled" '' | tr ' ' '#')" \
        "$(printf '%*s' "$empty" '' | tr ' ' '-')"
}

# Status glyph: nothing started, something started, everything done. Written as
# `if` blocks rather than the call sites' `[ … ] && icon=…` chain, which returns
# non-zero under `set -e` when the test fails.
plan_progress_icon() {
    local completed="$1" percent="$2" icon='💤'
    if [ "$completed" -gt 0 ]; then
        icon='⏳'
    fi
    if [ "$percent" -eq 100 ]; then
        icon='✅'
    fi
    printf '%s\n' "$icon"
}

# ── Progress rendering ───────────────────────────────────────────────────────
# Half-up rounding (+ total / 2) and the 20-column default width are contract:
# every caller must render byte-identical output.
plan_progress_percent() {
    local completed="$1" total="$2"
    if [ "$total" -gt 0 ]; then
        printf '%s\n' "$(( (completed * 100 + total / 2) / total ))"
    else
        printf '0\n'
    fi
}

# The label a progress table's Completion status cell carries, from the status
# word its command was given. Non-zero on an unknown word, so the caller keeps
# owning the usage message. The glyphs are the on-disk contract.
plan_status_label() {
    case "$1" in
        incomplete) printf '%s\n' '💤 incomplete' ;;
        in-progress|in_progress) printf '%s\n' '⏳ in progress' ;;
        completed) printf '%s\n' '✅ completed' ;;
        *) return 1 ;;
    esac
}

# Derive a row description from a step's Objective paragraph: the text after
# the first "§ N.N" label inside "## Objective", truncated to 100 chars. Falls
# back to "$2" so a progress table never carries a literal placeholder.
plan_step_objective() {
    local step_file="$1" fallback="$2"
    local desc
    desc="$(awk '
        /^## Objective$/ { in_obj = 1; next }
        /^§ [0-9]+\.[0-9]+$/ && in_obj { after_label = 1; next }
        after_label && NF {
            line = $0; sub(/^[[:space:]]+/, "", line)
            if (length(line) > 100) line = substr(line, 1, 100) "..."
            print line; exit
        }
        /^## / && in_obj { exit }
    ' "$step_file" 2>/dev/null)"
    [ -n "$desc" ] || desc="$fallback"
    printf '%s\n' "$desc"
}
