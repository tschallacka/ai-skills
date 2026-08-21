#!/usr/bin/env bash
# MODE: PROD
# plan-mutate.sh — the single dispatcher for every durable plan mutation.
#
# Each subcommand exec's the one helper that owns that mutation, so the protocol
# has exactly one entry point and direct edits to .plans stay prohibited. Two
# subcommands are implemented inline instead of dispatched — see the note above
# add_progress_step().
#
# Usage:
#   plan-mutate.sh <subcommand> [arguments...]
#   plan-mutate.sh --help
#
# Exit codes: whatever the dispatched helper returns; 64 for a bad subcommand.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage:
  ${0##*/} add-goal <plan> <goal-name> <title> <outcome>
  ${0##*/} add-work-unit <plan> <WNN> <type> <file|N/A> <scope> <subscope|N/A> <change> <depends-on|—> <goal> <step>
  ${0##*/} add-testing <goal-directory> <step-name> <verification-instructions>
  ${0##*/} add-progress <goal-directory> <step-name> <description>
  ${0##*/} rebuild-progress <goal-directory>
  ${0##*/} add-coverage <plan> <outcome-or-proof> <WNN[,WNN...]> <notes>
  ${0##*/} add-finding <plan> <AR-NN> <finding> <resolution> [open|in-progress|resolved]
  ${0##*/} set-unit-scope <plan> <WNN> <new-primary-scope>
  ${0##*/} remove-work-unit <plan> <WNN>
  ${0##*/} remove-plan <plan-directory>
  ${0##*/} cleanup-plans [--list] [<plan-name> ...] [--yes]
  ${0##*/} set-step <goal-directory> <step-name> <incomplete|in-progress|completed>
  ${0##*/} set-goal <plan> <goal-name> <incomplete|in-progress|completed>
  ${0##*/} set-review <plan> <pending|approved>
  ${0##*/} set-decomposition <plan> <incomplete|completed>
  ${0##*/} update-adversarial-review <plan> [--file CSV]
  ${0##*/} rebuild-plan-progress <plan>
  ${0##*/} validate <plan>

All durable plan mutations must use this dispatcher or the named helper it
dispatches. Direct edits to .plans are prohibited by the planning protocol.
USAGE
    exit "$rc"
}

[ "$#" -ge 1 ] || usage
case "$1" in -h|--help) usage 0 ;; esac
command="$1"
shift

# add-progress and rebuild-progress are implemented inline rather than exec'd:
# rebuild-progress must overwrite in place, tolerate an empty steps/ and stay
# silent, none of which the create path does.
add_progress_step() {
    [ "$#" -eq 3 ] || usage
    local goal_dir="$1" step_name="$2" description="$3" progress_file temporary
    source "$script_dir/plan-document-lib.sh"
    plan_require_directory "$goal_dir"
    [[ "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]] || plan_die "Step name must use 01-step-kebab-case"
    plan_require_safe_value description "$description"
    progress_file="$goal_dir/progress.md"
    [ -f "$progress_file" ] || plan_die "Progress file not found: $progress_file"
    plan_git_snapshot "$(dirname "$goal_dir")"
    ! grep -Fq "| $step_name |" "$progress_file" || plan_die "Progress row already exists: $step_name"
    temporary="${progress_file}.tmp.$$"
    trap 'rm -f "$temporary"' RETURN
    cp "$progress_file" "$temporary"
    printf '| %s | %s | %s | 💤 incomplete |\n' "$(basename "$goal_dir")" "$step_name" "$description" >> "$temporary"
    mv "$temporary" "$progress_file"
    trap - RETURN
}
rebuild_progress() {
    [ "$#" -eq 1 ] || usage
    local goal_dir="$1" goal_name progress_file temporary step_file step_name step_desc
    source "$script_dir/plan-document-lib.sh"
    plan_require_directory "$goal_dir"
    goal_name="$(basename "$goal_dir")"
    progress_file="$goal_dir/progress.md"
    plan_git_snapshot "$(dirname "$goal_dir")"
    temporary="${progress_file}.tmp.$$"
    trap 'rm -f "$temporary"' RETURN
    {
        printf '# Progress: %s\n\n' "$goal_name"
        # Glyphs and spacing are pinned by test-progress-bar-shape.sh; do not reflow.
        printf '**Progress:** `0%%  #### ----------------  100%%` 💤\n\n'
        printf '| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n'
        while IFS= read -r step_file; do
            step_name="$(basename "$step_file" .md)"
            # Derive the row description from the step's Objective (§ 4.1);
            # never a literal placeholder in a generated table.
            step_desc="$(plan_step_objective "$step_file" "$step_name")"
            printf '| %s | %s | %s | 💤 incomplete |\n' "$goal_name" "$step_name" "$step_desc"
        done < <(find "$goal_dir/steps" -maxdepth 1 -type f -name '*.md' ! -name '*-testing.md' -print | sort)
    } > "$temporary"
    mv "$temporary" "$progress_file"
    trap - RETURN
}
case "$command" in
    add-goal) exec "$script_dir/add-goal.sh" "$@" ;;
    add-work-unit) exec "$script_dir/add-work-unit.sh" "$@" ;;
    add-testing) exec "$script_dir/create-step-testing.sh" "$@" ;;
    add-progress) add_progress_step "$@" ;;
    add-coverage) exec "$script_dir/add-coverage.sh" "$@" ;;
    add-finding) exec "$script_dir/add-adversarial-finding.sh" "$@" ;;
    set-unit-scope) exec "$script_dir/update-work-unit.sh" "$@" ;;
    remove-work-unit) exec "$script_dir/remove-work-unit.sh" "$@" ;;
    remove-plan) exec "$script_dir/remove-plan.sh" "$@" ;;
    cleanup-plans) exec "$script_dir/cleanup-plans.sh" "$@" ;;
    rebuild-progress) rebuild_progress "$@" ;;
    set-step) exec "$script_dir/update-step.sh" "$@" ;;
    set-goal) exec "$script_dir/update-plan-progress.sh" "$@" ;;
    set-review) exec "$script_dir/update-plan-content.sh" --review-status "$@" ;;
    set-decomposition) exec "$script_dir/update-plan-content.sh" --decomposition-review "$@" ;;
    update-adversarial-review) exec "$script_dir/update-adversarial-review.sh" "$@" ;;
    rebuild-plan-progress) exec "$script_dir/rebuild-plan-progress.sh" "$@" ;;
    validate)
        [ "$#" -eq 1 ] || usage
        exec "$script_dir/validate-plan.sh" "$1"
        ;;
    *) usage ;;
esac
