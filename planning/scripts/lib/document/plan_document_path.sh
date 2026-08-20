#!/usr/bin/env bash
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
