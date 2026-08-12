#!/usr/bin/env bash
# Shared reconciliation helpers for plan mutations (sourced, not executed).
# Source plan-document-lib.sh first (provides plan_die, plan_replace_section, ...).
#
# These implement the "tools that reconcile references automatically" contract:
# when a work-unit mutates (add/remove), related artifacts — coverage rows, the
# goal's Owned work units section, and progress trackers — are updated here so
# agents do not have to issue follow-up calls.

set -euo pipefail

# Standard flag/arity guard. Accepts optional -h/--help (prints HELP, exit 0).
# Args: <expected-arg-count> <help-text>
plan_guard() {
    local expected="$1" help_text="$2" args=("${@:3}")
    if [ "${#args[@]}" -eq 1 ] && { [ "${args[0]}" = -h ] || [ "${args[0]}" = --help ]; }; then
        printf '%s\n' "$help_text"
        exit 0
    fi
    [ "${#args[@]}" -eq "$expected" ] || plan_die "$help_text"
}

# Actionable error: state the problem and what the agent can do.
plan_err() {
    printf 'plan: %s\n' "$1" >&2
    exit 64
}

# Prune a work-unit id from the inventory: drop its W row, remove the id from
# any coverage row (keeping other ids; deleting a row only when empty), and
# remove the id from remaining rows' "Depends on" column so no dangling
# dependency remains.
plan_prune_work_unit() {
    local inventory="$1" unit="$2" temporary
    [ -f "$inventory" ] || plan_err "work-unit inventory not found: $inventory"
    temporary="${inventory}.tmp.$$"
    trap 'rm -f "$temporary"' RETURN
    if ! awk -F'|' -v wanted="$unit" '
        BEGIN { OFS="|" }
        /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
            id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
            if (id == wanted) next
            # prune from Depends on ($8)
            deps=$8; gsub(/^[[:space:]]+|[[:space:]]+$/, "", deps)
            nd = split(deps, dp, ","); out = ""; changed = 0
            for (i = 1; i <= nd; i++) { p = dp[i]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", p); if (p == wanted) changed = 1; else { if (out != "") out = out ","; out = out p } }
            if (changed) $8 = (out == "" ? "—" : out)
            print; next
        }
        /^\|/ {
            ids=$3; gsub(/^[[:space:]]+|[[:space:]]+$/, "", ids)
            n = split(ids, parts, ",")
            out = ""; found = 0
            for (i = 1; i <= n; i++) { p = parts[i]; gsub(/^[[:space:]]+|[[:space:]]+$/, "", p); if (p == wanted) found = 1; else { if (out != "") out = out ","; out = out p } }
            if (!found) { print; next }
            if (out == "") { printf "plan: coverage row has no remaining ids after removing %s; row dropped\n", wanted > "/dev/stderr"; next }
            $3 = out
            print
            next
        }
        { print }
    ' "$inventory" > "$temporary"; then
        rm -f "$temporary"
        plan_err "could not prune $unit from $inventory (malformed inventory?)"
    fi
    mv "$temporary" "$inventory"
    trap - RETURN
}

# Collect one goal's remaining work-unit rows as "id<TAB>intended-change".
plan_goal_units() {
    local inventory="$1" goal="$2"
    awk -F'|' -v goal="$goal" '
        /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
            g = $9; gsub(/^[[:space:]]+|[[:space:]]+$/, "", g)
            if (g != goal) next
            id = $2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id)
            ch = $7; gsub(/^[[:space:]]+|[[:space:]]+$/, "", ch)
            gsub(/\t/, " ", ch)
            printf "%s\t%s\n", id, ch
        }
    ' "$inventory"
}

# Re-derive a goal's Owned work units section from the inventory. The goal
# template interleaves the "## Testing requirement" heading with the
# owned-work-unit paragraphs, so rebuild the whole region between "## Owned
# work units" and "## Goal-size exception": owned blocks first, then the
# testing-requirement heading + table (kept from the current file). This
# removes any stale/empty/truncated paragraph left by prior string-munging.
plan_rewrite_owned_work_units() {
    local goal_file="$1" inventory="$2" goal="$3" body_file idx id ch region
    local testing_row testing_heading separator
    [ -f "$goal_file" ] || plan_err "goal file not found: $goal_file"
    # Preserve the current testing-requirement row (e.g. "| yes | reason |").
    testing_row="$(awk '/^\|[[:space:]]*(yes|no)[[:space:]]*\|/{print; exit}' "$goal_file")"
    [ -n "$testing_row" ] || testing_row='| no | <rationale> |'
    testing_heading='## Testing requirement'
    separator='|---|---|'
    body_file="$(mktemp "${TMPDIR:-/tmp}/plan-owned.XXXXXX")"
    trap 'rm -f "$body_file"' RETURN
    idx=0
    {
        while IFS=$'\t' read -r id ch; do
            [ -n "$id" ] || continue
            idx=$((idx + 1))
            printf '§ 9.%d\n`%s` — %s\n\n' "$idx" "$id" "$ch"
        done < <(plan_goal_units "$inventory" "$goal")
        printf '%s\n\n' "$testing_heading"
        printf '| Test required | Rationale |\n%s\n%s\n' "$separator" "$testing_row"
    } > "$body_file"
    # Rebuild the region in one pass: print the "## Owned work units" heading,
    # then the re-derived body, then skip the old region until Goal-size
    # exception. The body intentionally omits the heading to avoid duplicating it.
    awk -v body_file="$body_file" '
        BEGIN { while ((getline line < body_file) > 0) body = body line "\n" }
        /^## Owned work units/ { print; printf "\n%s", body; in_region = 1; next }
        in_region && /^## Goal-size exception/ { in_region = 0 }
        in_region { next }
        { print }
    ' "$goal_file" > "${goal_file}.tmp.$$"
    mv "${goal_file}.tmp.$$" "$goal_file"
    rm -f "$body_file"
    trap - RETURN
}

# Rebuild a goal progress tracker from its step files (overwrites).
plan_rebuild_goal_progress() {
    local script_dir="$1" goal_dir="$2" goal="$3" progress_file
    progress_file="$goal_dir/progress.md"
    if [ -f "$progress_file" ]; then
        rm -f "$progress_file"
        "$script_dir/create-progress.sh" "$goal_dir" "$goal" >/dev/null 2>&1 || \
            printf 'plan: could not rebuild goal progress for %s\n' "$goal" >&2
    fi
}

# Rebuild the plan-level progress tracker.
plan_rebuild_plan_progress() {
    local script_dir="$1" plan_dir="$2"
    if [ -f "$plan_dir/progress.md" ]; then
        "$script_dir/rebuild-plan-progress.sh" "$plan_dir" >/dev/null 2>&1 || \
            printf 'plan: could not rebuild plan progress\n' >&2
    fi
}
