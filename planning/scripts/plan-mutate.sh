#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

usage() {
    cat >&2 <<'USAGE'
Usage:
  plan-mutate.sh add-goal <plan> <goal-name> <title> <outcome>
  plan-mutate.sh add-work-unit <plan> <WNN> <type> <file|N/A> <scope> <subscope|N/A> <change> <depends-on|—> <goal> <step>
  plan-mutate.sh add-testing <goal-directory> <step-name> <verification-instructions>
  plan-mutate.sh add-progress <goal-directory> <step-name> <description>
  plan-mutate.sh add-coverage <plan> <outcome-or-proof> <WNN[,WNN...]> <notes>
  plan-mutate.sh add-finding <plan> <AR-NN> <finding> <resolution> [open|in-progress|resolved]
  plan-mutate.sh set-unit-scope <plan> <WNN> <new-primary-scope>
  plan-mutate.sh remove-work-unit <plan> <WNN>
  plan-mutate.sh set-step <goal-directory> <step-name> <incomplete|in-progress|completed>
  plan-mutate.sh set-goal <plan> <goal-name> <incomplete|in-progress|completed>
  plan-mutate.sh set-review <plan> <pending|approved>
  plan-mutate.sh set-decomposition <plan> <incomplete|completed>
  plan-mutate.sh update-adversarial-review <plan> [--file CSV]
  plan-mutate.sh rebuild-plan-progress <plan>
  plan-mutate.sh validate <plan>

All durable plan mutations must use this dispatcher or the named helper it
dispatches. Direct edits to .plans are prohibited by the planning protocol.
USAGE
    exit 64
}

[ "$#" -ge 1 ] || usage
command="$1"
shift
add_progress_step() {
    [ "$#" -eq 3 ] || usage
    local goal_dir="$1" step_name="$2" description="$3" progress_file temporary
    source "$script_dir/plan-document-lib.sh"
    plan_require_directory "$goal_dir"
    [[ "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]] || plan_die "Step name must use 01-step-kebab-case"
    plan_require_safe_value description "$description"
    progress_file="$goal_dir/progress.md"
    [ -f "$progress_file" ] || plan_die "Progress file not found: $progress_file"
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
    local goal_dir="$1" goal_name progress_file temporary step_file step_name
    source "$script_dir/plan-document-lib.sh"
    plan_require_directory "$goal_dir"
    goal_name="$(basename "$goal_dir")"
    progress_file="$goal_dir/progress.md"
    temporary="${progress_file}.tmp.$$"
    trap 'rm -f "$temporary"' RETURN
    {
        printf '# Progress: %s\n\n' "$goal_name"
        printf '**Progress:** `0%%  #### ----------------  100%%` 💤\n\n'
        printf '| Goalname | Stepname | Description | Completion status |\n|---|---|---|---|\n'
        while IFS= read -r step_file; do
            step_name="$(basename "$step_file" .md)"
            printf '| %s | %s | <short description> | 💤 incomplete |\n' "$goal_name" "$step_name"
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
