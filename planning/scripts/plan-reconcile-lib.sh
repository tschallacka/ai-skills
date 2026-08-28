#!/usr/bin/env bash
# MODE: PROD
# Shared reconciliation helpers for plan mutations (sourced, not executed).
#
# These implement the "tools that reconcile references automatically" contract:
# when a work-unit mutates (add/remove), related artifacts — coverage rows, the
# goal's Owned work units section, and progress trackers — are updated here so
# agents do not have to issue follow-up calls.
# shellcheck disable=SC2154  # plan_inventory_* are assigned at runtime by the
# sourced plan-inventory-lib row/split helpers

set -euo pipefail

# A library that depends on another sources it itself rather than trusting the
# caller's order (CODE-STYLE §7). plan-document-lib.sh guards its own load-time
# initialisation, so sourcing it here is harmless when the caller already did.
if [ -z "${PLAN_DOCUMENT_LIB_INITIALISED:-}" ]; then
    # shellcheck source=planning/scripts/plan-document-lib.sh
    source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/plan-document-lib.sh"
fi


# Prune a work-unit id from the inventory: drop its W row, remove it from
# coverage rows (deleting a row only when it empties), and strip it from every
# remaining "Depends on" column so no dangling dependency remains.
# plan_prune_csv_cell LIST REMOVE — print LIST with the REMOVE entry cut out
# of the comma set (entries whitespace-trimmed, order preserved). The output
# equals the input when REMOVE is absent, so callers detect change by diff.
plan_prune_csv_cell() {
    local raw="$1" drop="$2" out="" p
    local -a parts=()
    IFS=',' read -r -a parts <<< "$raw"
    for p in ${parts[@]+"${parts[@]}"}; do
        p="${p#"${p%%[![:space:]]*}"}"
        p="${p%"${p##*[![:space:]]}"}"
        if [ "$p" = "$drop" ]; then continue
        fi
        [ -n "$out" ] && out="$out,"
        out="$out$p"
    done
    printf '%s' "$out"
}

plan_prune_work_unit() {
    local inventory="$1" unit="$2" temporary
    [ -f "$inventory" ] || plan_die "work-unit inventory not found: $inventory" 66
    temporary="${inventory}.tmp.$$"
    trap 'rm -f "$temporary"' RETURN
    if ! (
        # Unit rows: drop the removed unit, prune it from Depends (cell 8).
        # Other table rows: prune from the comma list in cell 3; a row whose
        # list empties is dropped with a stderr note, exactly as before.
        while IFS= read -r rline || [ -n "$rline" ]; do
            if [[ $rline =~ ^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\| ]]; then
                if [ "$(plan_table_cell "$rline" 2)" = "$unit" ]; then
                    continue
                fi
                deps_raw="$(plan_table_cell "$rline" 8)"
                out="$(plan_prune_csv_cell "$deps_raw" "$unit")"
                if [ "$out" != "$deps_raw" ]; then
                    [ -n "$out" ] || out="—"
                    rline="$(plan_table_set_cell "$rline" 8 "$out")"
                fi
            elif [[ $rline == \|* ]]; then
                ids_raw="$(plan_table_cell "$rline" 3)"
                out="$(plan_prune_csv_cell "$ids_raw" "$unit")"
                if [ "$out" != "$ids_raw" ]; then
                    # T59: a shrunk coverage row is named with what survives,
                    # exactly like a dropped one — the silent one-way shrink
                    # is what made remove+readd reads invisible (B68).
                    if [ -z "$out" ]; then
                        printf 'plan: coverage row has no remaining ids after removing %s; row dropped\n' "$unit" >&2
                        continue
                    fi
                    printf 'plan: pruned coverage id %s; remaining in row: %s\n' "$unit" "$out" >&2
                    rline="$(plan_table_set_cell "$rline" 3 "$out")"
                fi
            fi
            printf '%s\n' "$rline"
        done < "$inventory"
    ) > "$temporary"; then
        rm -f "$temporary"
        plan_die "could not prune $unit from $inventory (malformed inventory?)" 65
    fi
    mv "$temporary" "$inventory"
    trap - RETURN
}

# Collect one goal's remaining work-unit rows as "id<TAB>intended-change".
plan_goal_units() {
    local inventory="$1" goal="$2" row
    while IFS= read -r row; do
        plan_inventory_split "$row"
        [ "$plan_inventory_goal" = "$goal" ] || continue
        printf '%s\t%s\n' "$plan_inventory_id" "$plan_inventory_change"
    done < <(plan_inventory_rows "$inventory")
}

# Re-derive a goal's Owned work units section from the inventory. The template
# interleaves the "## Testing requirement" heading with the owned paragraphs, so
# rebuild the whole region up to "## Goal-size exception", not just the blocks.
# plan_owned_units_body BODY_FILE INVENTORY GOAL HEADING SEP ROW — write the
# re-derived Owned-work-units body (§ 9.x entries, emptied-roster placeholder,
# testing-requirement tail) for plan_rewrite_owned_work_units.
plan_owned_units_body() {
    local bfile="$1" inventory="$2" goal="$3" heading="$4" sep="$5" row="$6"
    local idx=0 id ch
    {
        while IFS=$'\t' read -r id ch; do
            [ -n "$id" ] || continue
            idx=$((idx + 1))
            printf '§ 9.%d\n`%s` — %s\n\n' "$idx" "$id" "$ch"
        done < <(plan_goal_units "$inventory" "$goal")
        # An emptied roster returns to the just-created shape rather than
        # vanishing: the placeholder is registered and is what the next
        # add-work-unit.sh replaces, so a goal stays addable after its last
        # unit goes.
        if [ "$idx" -eq 0 ]; then
            printf '§ 9.1\n<add work units with add-work-unit.sh>\n\n'
        fi
        printf '%s\n\n' "$heading"
        printf '| Test required | Rationale |\n%s\n%s\n' "$sep" "$row"
    } > "$bfile"
}

plan_rewrite_owned_work_units() {
    local goal_file="$1" inventory="$2" goal="$3" body_file idx id ch region
    local testing_row testing_heading separator
    [ -f "$goal_file" ] || plan_die "goal file not found: $goal_file" 66
    # Preserve the current testing-requirement row (e.g. "| yes | reason |").
    testing_row="$(plan_testing_requirement_row "$goal_file")"
    [ -n "$testing_row" ] || testing_row='| no | <rationale> |'
    testing_heading='## Testing requirement'
    separator='|---|---|'
    body_file="$(mktemp "${TMPDIR:-/tmp}/plan-owned.XXXXXX")"
    trap 'rm -f "$body_file"' RETURN
    plan_owned_units_body "$body_file" "$inventory" "$goal" \
        "$testing_heading" "$separator" "$testing_row"
    # Rebuild the region in one pass: print the "## Owned work units" heading,
    # then the re-derived body, then skip the old region until Goal-size
    # exception. The body intentionally omits the heading to avoid duplicating it.
    # The rebuilt body carries the goal's one Testing requirement section, so
    # every ORIGINAL such section is dropped wherever it sits — goals differ on
    # whether it lives between Owned work units and Goal-size exception or
    # after it (found when the B68 move duplicated it; the old skip-only
    # region assumed one order and emitted two sections on the other).
    awk -v body_file="$body_file" '
        BEGIN { while ((getline line < body_file) > 0) body = body line "\n" }
        /^## Testing requirement/ { in_testing = 1; next }
        in_testing && /^## / { in_testing = 0 }
        in_testing { next }
        /^## Owned work units/ { print; printf "\n%s", body; in_region = 1; next }
        in_region && /^## Goal-size exception/ { in_region = 0 }
        in_region { next }
        { print }
    ' "$goal_file" > "${goal_file}.tmp.$$"
    mv "${goal_file}.tmp.$$" "$goal_file"
    rm -f "$body_file"
    trap - RETURN
}

# Rebuild a goal progress tracker from its step files. Existing statuses carry
# across by step name so an add never resets completed work (B40); new rows
# start incomplete; rows for removed steps are dropped.
plan_rebuild_goal_progress() {
    local script_dir="$1" goal_dir="$2" goal="$3" progress_file
    progress_file="$goal_dir/progress.md"
    if [ -f "$progress_file" ]; then
        local saved="$progress_file.pre-rebuild.$$"
        cp "$progress_file" "$saved"
        rm -f "$progress_file"
        "$script_dir/create-progress.sh" "$goal_dir" "$goal" >/dev/null 2>&1 || \
            { printf 'plan: could not rebuild goal progress for %s\n' "$goal" >&2; mv "$saved" "$progress_file"; return; }
        # Carry each old status onto the matching new row by step name.
        status_pairs=""
        while IFS= read -r orow || [ -n "$orow" ]; do
            case "$orow" in '|'*) ;; *) continue ;; esac
            case "$orow" in *'---'*|*Goalname*|*'Progress:'*) continue ;; esac
            status_pairs="$status_pairs$(plan_table_cell "$orow" 3)"$'\t'"$(plan_table_cell "$orow" 5)"$'\n'
        done < "$saved"
        while IFS= read -r nrow || [ -n "$nrow" ]; do
            case "$nrow" in '|'*)
                case "$nrow" in *'---'*|*Goalname*|*'Progress:'*) ;; *)
                    nkey="$(plan_table_cell "$nrow" 3)"
                    while IFS=$'\t' read -r skey sstat; do
                        [ "$skey" = "$nkey" ] || continue
                        nrow="${nrow//💤 incomplete/$sstat}"
                        break
                    done <<< "$status_pairs"
                    ;;
                esac ;;
            esac
            printf '%s\n' "$nrow"
        done < "$progress_file" | plan_atomic_write "$progress_file"
        rm -f "$saved"
        printf 'note: goal progress was rebuilt from step files; existing statuses carried across where step names match\n' >&2
    elif [ -n "$(find "$goal_dir/steps" -maxdepth 1 -type f -name '*.md' \
        ! -name '*-testing.md' -print -quit 2>/dev/null || true)" ]; then
        "$script_dir/create-progress.sh" "$goal_dir" "$goal" >/dev/null 2>&1 || \
            printf 'plan: could not create goal progress for %s\n' "$goal" >&2
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
