#!/usr/bin/env bash
# MODE: PROD
# validate-plan-propagation-lib.sh — the completion gate plus the propagation
# checks: the surfaces of a work unit must agree. A finding cites one surface; a
# fix must reach the others, and this is the mechanical part of that contract.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh and the inventory data model.
#
# Passes, in the order the entry script runs them:
#   plan_validate_completion               --complete: progress trackers agree
#   plan_validate_propagation_symbols  (a) an instructed edit to an unowned symbol
#   plan_validate_propagation_reach    (c) a verifier must reach what it grades
#   plan_validate_propagation_companion (c2) a companion's unit references
#   plan_validate_propagation_leaves   (d) unverified graph leaves
#   plan_validate_propagation_roster   (e) §9.x roster vs inventory
# (b) was removed by report 7 and is documented at its old site below.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

plan_validate_completion() {
    if [ "$complete_mode" = true ]; then
        plan_progress="$plan_dir/progress.md"
        if [ ! -f "$plan_progress" ]; then
            fail "Completion requires plan-level progress.md"
        else
            while IFS= read -r goal_name; do
                [ -n "$goal_name" ] || continue
                goal_status="$(awk -F'|' -v wanted="$goal_name" '
                    /^\|/ { key=$2; status=$4; gsub(/^[[:space:]]+|[[:space:]]+$/, "", key); gsub(/^[[:space:]]+|[[:space:]]+$/, "", status); if (key == wanted) print status }
                ' "$plan_progress")"
                [ "$goal_status" = '✅ completed' ] || fail "$goal_name is not completed in plan progress"
            done < <(plan_map_keys goal_units)
        fi
        for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
            plan_map_load unit_goal "$id" || plan_map_value=""
            u_goal="$plan_map_value"
            plan_map_load unit_step "$id" || plan_map_value=""
            u_step="$plan_map_value"
            goal_progress="$plan_dir/$u_goal/progress.md"
            if [ ! -f "$goal_progress" ]; then
                fail "$id completion requires $goal_progress"
                continue
            fi
            step_status="$(awk -F'|' -v wanted="$u_step" '
                /^\|/ { key=$3; status=$5; gsub(/^[[:space:]]+|[[:space:]]+$/, "", key); gsub(/^[[:space:]]+|[[:space:]]+$/, "", status); if (key == wanted) print status }
            ' "$goal_progress")"
            [ "$step_status" = '✅ completed' ] || fail "$id is not completed in $u_goal progress"
        done
    fi
}

# --- propagation checks (--propagation): the surfaces of a work unit must
#     agree. A finding cites one surface; a fix must reach the others, and this
#     is the mechanical part of that contract. ---
plan_validate_propagation_symbols() {
    # (a) Flag a ::-symbol or path on an edit-intent line when its namespace
    #     root or path prefix is one the plan itself edits, no inventory row
    #     owns it, and the line instructs an edit. Vendor seams drop out.
    declare -a project_prefixes=()
    for candidate in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_load unit_file "$candidate" || plan_map_value=""
        fc="$plan_map_value"
        [ -n "$fc" ] && [ "$fc" != "N/A" ] || continue
        # File column forms: Namespace\Class.php, app/code/V/M/File.php,
        # app/design/.../file.phtml, path/to/file.php. Derive the namespace
        # root or the leading directory segments.
        case "$fc" in
            *'\\'*)
                ns_root="${fc%%\\*}"
                ;;
            app/*|vendor/*)
                ns_root="${fc%%/*}"
                ;;
            *)
                ns_root="$(dirname "$fc" 2>/dev/null)"
                ;;
        esac
        [ -n "$ns_root" ] && project_prefixes+=("$ns_root")
    done
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_load unit_goal "$id" || plan_map_value=""
        u_goal="$plan_map_value"
        plan_map_load unit_step "$id" || plan_map_value=""
        u_step="$plan_map_value"
        step_file="$plan_dir/$u_goal/steps/$u_step.md"
        [ -f "$step_file" ] || continue
        instr_section="$(awk '
            /^## Instructions$/ { in_sec = 1; next }
            /^## / && in_sec { exit }
            in_sec { print }
        ' "$step_file")"
        [ -n "$instr_section" ] || continue
        # Edit/create-intent lines: the symbol must sit on a line that also
        # instructs an edit (create, add, implement, edit, change, update,
        # modify, rewrite, replace, override).
        edit_lines="$(printf '%s' "$instr_section" | grep -iE '(create|add|implement|edit|change|update|modify|rewrite|replace|override)' || true)"
        [ -n "$edit_lines" ] || continue
        # Well-formed Class::method tokens only. X::class and Vendor_Module::path
        # are excluded: the tokeniser would truncate them into a plausible but
        # nonexistent Class::method.

        # PORTABILITY(ere-word-boundary): `tr` yields maximal word runs, so a
        # run starting with the token means exactly what a leading \b meant.
        tokens="$(printf '%s' "$edit_lines" | tr -c 'A-Za-z0-9_\\:' '\n' \
            | grep -oE '^[A-Z][A-Za-z0-9_]*(\\[A-Za-z_][A-Za-z0-9_]*)*::[A-Za-z_][A-Za-z0-9_]*' \
            | sort -u || true)"
        for token in $tokens; do
            # Skip X::class — a PHP class constant (Foo::class, Bar::class),
            # not a method call, and not an edit target.
            [ "${token##*::}" = "class" ] && continue
            # Template-identifier guard: Vendor_Module::<path> (Magento_Weee::
            # email/items/price/row.phtml) is a template id, not a class method.
            if [[ "$token" =~ ^[A-Z][a-zA-Z0-9]*_[A-Z][a-zA-Z0-9]*:: ]]; then
                # Confirm the full source line carries a path after ::(a slash).
                # PORTABILITY(pipefail-grep-q): grep -c drains the pipe, and the
                # match must stay line-scoped, which a bash =~ would not be.
                if printf '%s' "$edit_lines" \
                    | grep -cE "${token%%::*}[A-Za-z0-9_]*::[^ (]*/" >/dev/null; then
                    continue
                fi
            fi
            # Condition 1: the symbol's namespace root must be a project prefix
            # the plan edits (vendor seams drop out here by construction).
            klass="${token%%::*}"
            klass_short="${klass##*\\}"
            prefix_match=false
            # PORTABILITY(empty-array-setu)
            for prefix in ${project_prefixes[@]+"${project_prefixes[@]}"}; do
                case "$klass" in
                    "$prefix"*|"$klass_short") prefix_match=true; break ;;
                esac
                [[ "$klass_short" == "$prefix"* ]] && { prefix_match=true; break; }
            done
            [ "$prefix_match" = true ] || continue
            # Condition 2: no inventory row owns it (file basename or scope).
            owned=false
            for candidate in ${unit_ids[@]+"${unit_ids[@]}"}; do
                plan_map_load unit_file "$candidate" || plan_map_value=""
                file_cell="$plan_map_value"
                plan_map_load unit_scope "$candidate" || plan_map_value=""
                scope_cell="$plan_map_value"
                scope_class="${scope_cell%%::*}"
                scope_class_short="${scope_class##*\\}"
                [ "$(basename "$file_cell" 2>/dev/null)" = "$klass_short" ] && { owned=true; break; }
                [ "$file_cell" = "$klass" ] && { owned=true; break; }
                [ "$scope_class" = "$klass" ] && { owned=true; break; }
                [ "$scope_class_short" = "$klass_short" ] && { owned=true; break; }
            done
            # From text alone this cannot separate "edit this" from "this is
            # where we attach", and the short class form carries no namespace.
            # Deliberately a WARN: as a FAIL the heuristic would block plans.
            if [ "$owned" = false ] && [ "$token" != "$id" ]; then
                warn "$id instructions mention '$token' which no inventory row owns; verify it is a seam description, or add a discovery/ownership row if it is an edit target"
            fi
        done
        # (b) Removed by report 7: cross-mention warnings fired on any passing
        #     sibling reference ("W83 owns this payload, do not duplicate it")
        #     and produced 500+ warnings that penalised the seven-surface prose.
    done
}

    # (c) A verification unit must reach, transitively, every same-goal unit it
    #     grades. Reverse edges are deliberate (a baseline-capture verification
    #     runs FIRST), so an opposite-direction edge is a guard, not a violation.

# Recursive: do not rename. `dep_seen` marks pairs whose walk is in progress
# and cuts cycles; `dep_failed` caches pairs proven unreachable after a
# complete exploration, so an earlier success can never read back as one.
dep_reaches() {
    local from="$1" to="$2" dep key
    [ "$from" = "$to" ] && return 0
    key="$from/$to"
    plan_map_has dep_failed "$key" && return 1
    plan_map_has dep_seen "$key" && return 1
    plan_map_set dep_seen "$key" 1
    while IFS= read -r dep; do
        [ -n "$dep" ] || continue
        [ "$dep" = "$to" ] && return 0
        dep_reaches "$dep" "$to" && return 0
    done < <(plan_map_load unit_depends "$from" || true; printf '%s' "$plan_map_value" | grep -oE 'W[0-9][0-9]+' || true)
    plan_map_set dep_failed "$key" 1
    return 1
}

plan_validate_propagation_reach() {
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_load unit_type "$id" || plan_map_value=""
        [ "$plan_map_value" = verification ] || continue
        plan_map_load unit_goal "$id" || plan_map_value=""
        u_goal="$plan_map_value"
        plan_map_load unit_step "$id" || plan_map_value=""
        u_step="$plan_map_value"
        step_file="$plan_dir/$u_goal/steps/$u_step.md"
        [ -f "$step_file" ] || continue
        # PORTABILITY(ere-word-boundary): a word run equal to the ID is `\bWnn\b`.
        named_units="$(tr -c 'A-Za-z0-9_' '\n' < "$step_file" | grep -xE 'W[0-9][0-9]+' | sort -u)" || true
        for named in $named_units; do
            [ "$named" = "$id" ] && continue
            # A WNN not in this plan's inventory is a cross-plan reference
            # ("the extended-rendering plan's W04"), correct prose, not a typo.
            if ! plan_map_has unit_type "$named"; then
                continue
            fi
            plan_map_load unit_goal "$named" || plan_map_value=""
            if [ "$plan_map_value" = "$u_goal" ]; then
                plan_map_clear dep_seen
                plan_map_clear dep_failed
                if dep_reaches "$id" "$named"; then
                    : # already transitively ordered
                elif dep_reaches "$named" "$id"; then
                    : # reverse edge is a deliberate ordering (e.g. baseline capture)
                else
                    fail "$id is a verification unit that grades $named but has no dependency path to it; add a dependency edge"
                fi
            fi
        done
    done
}

# (c2) A testing companion must not reference work units the step does not own
#      or depend on: the executor runs the companion, so a stale unit reference
#      there directs execution at the wrong target.
plan_validate_propagation_companion() {
    for id in ${unit_ids[@]+"${unit_ids[@]}"}; do
        plan_map_load unit_goal "$id" || plan_map_value=""
        u_goal="$plan_map_value"
        plan_map_load unit_step "$id" || plan_map_value=""
        u_step="$plan_map_value"
        companion="$plan_dir/$u_goal/steps/$u_step-testing.md"
        [ -f "$companion" ] || continue
        plan_map_load unit_depends "$id" || plan_map_value=""
        deps="$(printf '%s' "$plan_map_value" | grep -oE 'W[0-9][0-9]+' | sort -u | tr '\n' ' ')" || true
        # PORTABILITY(ere-word-boundary)
        named_units="$(tr -c 'A-Za-z0-9_' '\n' < "$companion" | grep -xE 'W[0-9][0-9]+' | sort -u)" || true
        for named in $named_units; do
            [ "$named" = "$id" ] && continue
            # Cross-plan reference, not a typo: a WNN outside this plan is prose.
            if ! plan_map_has unit_type "$named"; then
                continue
            fi
            # A companion may correctly reference a same-goal test/verification
            # unit ("automated tests: covered by WNN") — that is proof-coverage
            # prose, not a dependency claim (report 14 §5). Skip it.
            plan_map_load unit_goal "$named" || plan_map_value=""
            named_goal="$plan_map_value"
            plan_map_load unit_type "$named" || plan_map_value=""
            named_type="$plan_map_value"
            if [ "$named_goal" = "$u_goal" ] && \
               { [ "$named_type" = test ] || [ "$named_type" = verification ]; }; then
                continue
            fi
            case "$deps" in
                *"$named"*) ;;
                *) warn "$id companion references $named, which $id neither owns nor depends on; update the companion or add the dependency edge" ;;
            esac
        done
    done
}

# (d) Graph leaves in a goal that owns a verification unit: a non-verification
#     unit nothing depends on is unverified work.
plan_validate_propagation_leaves() {
    while IFS= read -r goal_name; do
        [ -n "$goal_name" ] || continue
        goal_has_verifier=false
        plan_map_load goal_units "$goal_name" || plan_map_value=""
        goal_unit_ids="$plan_map_value"
        for id in $goal_unit_ids; do
            plan_map_load unit_type "$id" || plan_map_value=""
            [ "$plan_map_value" = verification ] && goal_has_verifier=true
        done
        [ "$goal_has_verifier" = true ] || continue
        for id in $goal_unit_ids; do
            plan_map_load unit_type "$id" || plan_map_value=""
            [ "$plan_map_value" = verification ] && continue
            dependent=false
            for candidate in ${unit_ids[@]+"${unit_ids[@]}"}; do
                plan_map_load unit_depends "$candidate" || plan_map_value=""
                case "$plan_map_value" in
                    *"$id"*) dependent=true; break ;;
                esac
            done
            if [ "$dependent" = false ]; then
                warn "$id is a graph leaf in a goal that owns a verification unit; nothing depends on it, so nothing verifies its output"
            fi
        done
    done < <(plan_map_keys goal_units)
}

# (e) Roster vs inventory: a goal's §9.x Owned-work-units roster must be
#     exactly the set the inventory assigns to it. The authoritative roster is
#     §9.1, not the §9.2+ blurbs, which need not exist for every unit.
plan_validate_propagation_roster() {
    while IFS= read -r goal_name; do
        [ -n "$goal_name" ] || continue
        goal_file="$plan_dir/$goal_name/goal.md"
        [ -f "$goal_file" ] || continue
        # Assigned set: the inventory's units for this goal.
        plan_map_load goal_units "$goal_name" || plan_map_value=""
        assigned="$(for id in $plan_map_value; do printf '%s ' "$id"; done)"
        roster_ids="$(awk -v assigned=" $assigned " '
            # Capture the §9.1 paragraph (the line after the § 9.1 label, until
            # the next § label or heading).
            /^§ 9\.1$/ { in_91 = 1; next }
            in_91 && /^§ / { in_91 = 0 }
            in_91 && /^## / { in_91 = 0 }
            in_91 && !/^[[:space:]]*$/ {
                para = para " " $0
            }
            END {
                if (para != "") {
                    # Leading run: everything before em-dash / " - " / period /
                    # "in that order". Split on those and take the head.
                    head = para
                    sub(/ —.*/, "", head)
                    sub(/ - .*/, "", head)
                    sub(/\..*/, "", head)
                    sub(/, in that order.*/, "", head)
                    sub(/ in that order.*/, "", head)
                    # Bare WNN in the leading run.
                    n = split(head, parts, /[ ,]+/)
                    for (i = 1; i <= n; i++) {
                        if (parts[i] ~ /^W[0-9][0-9]+$/) print parts[i]
                    }
                }
            }
        ' "$goal_file")"
        # Add the id that heads each per-unit paragraph. Only the leading
        # backticked id: stripping every backtick and harvesting the whole line
        # also collected ids the description legitimately cross-references ("as
        # `W05` does"), and the roster check then failed the goal for a unit it
        # never claimed to own. The em-dash exclusion noted below applies to the
        # leading-run pass above, not to this one.
        blurb_ids="$(awk '
            /^## Owned work units$/ { in_section = 1; next }
            /^## Goal-size exception$/ { in_section = 0 }
            in_section && /^`W[0-9][0-9]+`/ {
                after_tick = substr($0, 2)
                id = substr(after_tick, 1, index(after_tick, "`") - 1)
                if (id ~ /^W[0-9][0-9]+$/) print id
            }
        ' "$goal_file")"
        # An empty match is a real answer here, not an error: grep's exit 1
        # under pipefail would abort the whole run before any verdict prints.
        roster_ids="$(printf '%s\n%s\n' "$roster_ids" "$blurb_ids" | { grep -E '^W[0-9][0-9]+$' || true; } | sort -u | tr '\n' ' ')"
        # Roster-only units: a roster id the inventory does not assign to this
        # goal. Ids not in the plan at all are skipped; cross-plan references
        # sit after the em-dash and are already out of the leading run.
        for rid in $roster_ids; do
            if ! plan_map_has unit_type "$rid"; then
                continue
            fi
            case " $assigned " in
                *" $rid "*) : ;;
                *) fail "$goal_name §9.x roster lists $rid which the inventory does not assign to this goal; reconcile the roster and the inventory" ;;
            esac
        done
        # Assigned units missing from the roster.
        for aid in $assigned; do
            case " $roster_ids " in
                *" $aid "*) : ;;
                *) fail "$goal_name §9.x roster omits $aid which the inventory assigns to this goal; add it to the roster" ;;
            esac
        done
    done < <(plan_map_keys goal_units)
}
