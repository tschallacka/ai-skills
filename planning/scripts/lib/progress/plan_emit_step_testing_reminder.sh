#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: PROD
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
