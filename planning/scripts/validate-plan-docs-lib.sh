#!/usr/bin/env bash
# validate-plan-docs-lib.sh — the document-level gates: the required inputs
# exist, plan-description.md carries every mandated heading and a valid
# `UI affected` verdict, the adversarial review is present and its verdict is
# mirrored, the inventory has its coverage/work-unit/decomposition sections,
# and no plan document carries hand-edit damage (helper-flag-shaped text,
# duplicate paragraph labels, shell-variable path fragments).
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh.
#
# Publishes two globals the later passes read:
#   ui_affected — `yes` or `no` from the UI classification section.
#   plan_docs   — the authored-document list (plan description, review,
#                 goal.md files, non-testing step files, inventory). The
#                 placeholder and stale passes both iterate it; the stale pass
#                 extends it with the *-testing.md companions.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

# The obsolescence gate runs before the existence gate: a plan built by an older
# skill version must not be validated, migrated, or resumed, and reporting its
# findings would invite exactly that. Returns 65 when the marker is present, and
# never exits — the entry script owns process exit.
# ---- quoted: the OBSOLETE marker ----
# obsoleted-at: 2026-08-19
# obsoleted-because: built by an older planning-skill version
# replaced-by: 2026-08-19-checkout-rewrite
# ---- end quoted ----
plan_validate_obsolete() {
    local marker="$plan_dir/OBSOLETE" replacement
    [ -f "$marker" ] || return 0
    replacement="$(sed -nE 's/^[[:space:]]*replaced-by:[[:space:]]*(.*[^[:space:]])[[:space:]]*$/\1/p' \
        "$marker" | head -1)"
    printf '%s: %s is marked obsolete by its OBSOLETE marker: it was built by an older planning-skill version and is never validated, resumed, or migrated.\n' \
        "${0##*/}" "$plan_dir" >&2
    if [ -n "$replacement" ]; then
        printf '%s: the initiative was rebuilt as: %s\n' "${0##*/}" "$replacement" >&2
    else
        printf '%s: the marker names no replacement; add a "replaced-by: <plan directory>" line naming the plan that supersedes this one.\n' \
            "${0##*/}" >&2
    fi
    printf '%s: nothing here was deleted — this directory is kept as history. Validate the replacement instead.\n' \
        "${0##*/}" >&2
    return 65
}

# The existence gate runs before every other pass: everything else reads these
# two files. Returns 66 when the plan directory is absent, 1 when a required
# document is, and never exits — the entry script owns process exit.
plan_validate_existence() {
    if [ ! -d "$plan_dir" ]; then
        printf 'Plan directory not found: %s\n' "$plan_dir" >&2
        return 66
    fi
    if [ ! -f "$plan_dir/plan-description.md" ]; then
        fail "Missing plan-description.md"
    fi
    if [ ! -f "$inventory" ]; then
        fail "Missing work-unit-inventory.md"
    fi
    if [ "$errors" -gt 0 ]; then
        return 1
    fi
    return 0
}

plan_validate_plan_docs() {
    for heading in \
        '## Current state' \
        '## Desired outcome' \
        '## Approach' \
        '## Approach decisions' \
        '## Scope' \
        '## Affected areas' \
        '## Constraints and decisions' \
        '## Risks and open questions' \
        '## Environment facts' \
        '## UI classification' \
        '## Adversarial review'; do
        require_heading "$plan_dir/plan-description.md" "$heading"
    done
    get_single_field "$plan_dir/plan-description.md" 'UI affected'; ui_affected="$field_value"
    case "$ui_affected" in
        yes|no) ;;
        *) fail "UI classification must declare '- UI affected: yes' or 'no'" ;;
    esac

    review_file="$plan_dir/adversarial-review.md"
    if [ ! -f "$review_file" ]; then
        fail "Missing adversarial-review.md"
    else
        require_heading "$review_file" '## Review scope'
        require_heading "$review_file" '## Findings'
        require_heading "$review_file" '## Verdict'
        grep -Fqx -- '- Status: `✅ approved`' "$review_file" || review_approved=false
        if [ "${review_approved:-true}" = true ]; then
            grep -Fqx -- '- Status: ✅ approved' "$plan_dir/plan-description.md" || fail "Plan description does not mirror approved adversarial-review status"
            if grep -Eq '^\|[[:space:]]*AR-[0-9]+[[:space:]]*\|.*\|[[:space:]]*(💤 open|⏳ in progress)[[:space:]]*\|' "$review_file"; then
                fail "Adversarial review has unresolved findings"
            fi
        else
            if grep -Fqx -- '- Status: ✅ approved' "$plan_dir/plan-description.md"; then
                fail "Plan description claims approval but adversarial review is not approved"
            fi
            if [ "$complete_mode" = true ]; then
                fail "Adversarial review is not approved"
            else
                warn "Adversarial review is not approved (expected mid-cycle; use validate-plan.sh --complete for the strict gate)"
            fi
        fi
    fi

    require_heading "$inventory" '## Definition-of-done coverage'
    require_heading "$inventory" '## Work units'
    require_heading "$inventory" '## Decomposition review'

    if grep -Eq '^[[:space:]]*-[[:space:]]*\[[^xX]\]' "$inventory"; then
        fail "Decomposition review contains unchecked items"
    fi
    for review in \
        '- [x] Every definition-of-done item maps to one or more work units.' \
        '- [x] Every known affected file and changing symbol has its own work unit.' \
        '- [x] Every work unit has exactly one goal and one step.' \
        '- [x] Each goal has 2–10 work units, or records an allowed exception.' \
        '- [x] Each step has exactly one work unit and no unnamed incidental edits.' \
        '- [x] Dependencies form an executable order with no cycle.'; do
        grep -Fqx -- "$review" "$inventory" || fail "Missing completed decomposition review: $review"
    done
    if grep -qi 'TBD' "$inventory"; then
        fail "Inventory contains TBD; add a bounded discovery work unit instead"
    fi

    # --- defect-report hardening: helper-flag-shaped text, duplicate paragraph
    #     labels, and path-like shell fragments must never appear in plan docs ---
    swallowed_flag_regex='(^|[[:space:]])-(p|dp|gp|sp|rp|tp|ia|ib)[[:space:]]+[0-9]+\.[0-9]+[[:space:]]*:'
    plan_docs=("$plan_dir/plan-description.md" "$plan_dir/adversarial-review.md")
    while IFS= read -r -d '' goal_file; do
        plan_docs+=("$goal_file")
        while IFS= read -r -d '' step_file; do
            plan_docs+=("$step_file")
        done < <(find "$(dirname "$goal_file")/steps" -maxdepth 1 -name '*.md' -not -name '*-testing.md' -print0 2>/dev/null)
    done < <(find "$plan_dir" -mindepth 2 -maxdepth 2 -name goal.md -print0 2>/dev/null)
    plan_docs+=("$inventory")
    for doc in "${plan_docs[@]}"; do
        [ -f "$doc" ] || continue
        if grep -Eq "$swallowed_flag_regex" "$doc"; then
            fail "$(basename "$doc") contains helper-flag-shaped text (-p N.N: etc.); mutate plan documents through the helpers, never by hand"
        fi
        duplicate_label="$(grep -E '^§ [0-9]+\.[0-9]+$' "$doc" | sort | uniq -d | head -1)" || true
        if [ -n "$duplicate_label" ]; then
            fail "$(basename "$doc") has duplicate paragraph label $duplicate_label; renumber through the helpers"
        fi
        if grep -Eq '(\$script_dir/|\$PLANNING_SKILL_DIR/)' "$doc"; then
            fail "$(basename "$doc") contains a shell-variable path fragment; bind file paths to the plan, not to script internals"
        fi
    done
}
