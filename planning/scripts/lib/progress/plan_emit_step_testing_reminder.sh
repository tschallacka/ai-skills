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
    if [ ! -f "$companion" ]; then
        printf 'Reminder: this goal requires testing; continue with its test/proof step before marking the goal complete.\n' >&2
        return 0
    fi
    plan_companion_is_behind "$plan_dir" "$step_file" "$companion" || return 0
    printf 'Reminder: %s was already behind %s before this edit; review it for accuracy and completeness.\n' \
        "${companion##*/}" "${step_file##*/}" >&2
}

# plan_companion_is_behind PLAN_DIR STEP COMPANION -- true when the companion
# was already older than the step BEFORE the current call wrote anything.
#
# mtime cannot answer this: the reminder runs after the write, so the step is
# always the newer file and the check would fire on every edit -- which is what
# made the old unconditional line worthless (T67). Every mutating helper commits
# the pre-mutation state first, so HEAD is the tree as it stood before this call
# and git can answer it exactly. A plan with no usable history stays silent: an
# always-firing reminder carries no information, so under-reporting beats it.
plan_companion_is_behind() {
    local plan_dir="$1" step="$2" companion="$3" step_at companion_at
    git -C "$plan_dir" rev-parse --git-dir >/dev/null 2>&1 || return 1
    step_at="$(git -C "$plan_dir" log -1 --format=%ct -- "${step#"$plan_dir"/}" 2>/dev/null)"
    companion_at="$(git -C "$plan_dir" log -1 --format=%ct -- "${companion#"$plan_dir"/}" 2>/dev/null)"
    [ -n "$step_at" ] && [ -n "$companion_at" ] || return 1
    [ "$companion_at" -lt "$step_at" ]
}
