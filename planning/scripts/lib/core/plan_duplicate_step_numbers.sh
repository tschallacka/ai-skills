#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: DEV
# Every duplicated step number in a plan, as `<goal> <number> <file> <file>...`,
# one line per collision. Empty output means the plan is clean.
#
# Two steps numbered 04 in one goal have no defined order: steps are read in
# number order and nothing breaks the tie. add-work-unit.sh refuses to create
# one, so this finds plans that already hold the state -- created before that
# guard, or by an editor working outside the helpers.
#
# Deliberately silent about gaps. A goal numbered 01, 04, 07 is unambiguous, so a
# missing number is not a defect and warning about it would train the reader to
# skip these lines.
plan_duplicate_step_numbers() {
    local plan_dir="$1" goal_dir goal steps_dir
    [ -d "$plan_dir" ] || return 0
    for goal_dir in "$plan_dir"/*/; do
        [ -d "$goal_dir" ] || continue
        steps_dir="${goal_dir}steps"
        [ -d "$steps_dir" ] || continue
        goal="$(basename "$goal_dir")"
        # The companion shares its step's number by design, so it is excluded
        # before counting rather than reported as a collision.
        ls "$steps_dir" 2>/dev/null \
            | grep -E '^[0-9][0-9]-step-.*\.md$' \
            | grep -v -- '-testing\.md$' \
            | awk -v goal="$goal" '
                { number = substr($0, 1, 2); files[number] = files[number] " " $0; seen[number]++ }
                END {
                    for (number in seen) {
                        if (seen[number] > 1) print goal " " number files[number]
                    }
                }' \
            | LC_ALL=C sort
    done
}
