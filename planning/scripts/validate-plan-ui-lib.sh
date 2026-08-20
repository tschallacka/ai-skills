#!/usr/bin/env bash
# MODE: PROD
# PACKAGE: PROD
# validate-plan-ui-lib.sh — the UI-validation surface: each user story records a
# real direct interaction and no console/state/direct-API shortcut, its browser
# run cache exists and is shaped correctly, its status is from the vocabulary
# and (under --complete) is terminal, and every bug-found story is linked to an
# investigation and a fix goal in bugs.md.
#
# Sourced by validate-plan.sh; never executed. Requires
# validate-plan-common-lib.sh and the inventory data model. Reads `ui_affected`
# from validate-plan-docs-lib.sh.

# shellcheck disable=SC2154
# The entry script and its sibling libs publish these globals; shellcheck lints
# each file alone and cannot see the assignments.
set -euo pipefail

interaction_pattern='[Cc]lick|[Tt]ap|[Tt]ype|[Kk]eyboard|[Pp]ress|[Ss]wipe|[Pp]inch|[Dd]rag|[Ss]elect'
prohibited_pattern='[Ee]valuate\(|[Dd]ev[Tt]ools|[Ii]nject|[Ll]ocal[Ss]torage|[Ss]ession[Ss]torage|[Xx][Mm][Ll][Hh][Tt][Tt][Pp][Rr]equest|[Ff]etch\(|[Cc]url|[Dd]irect[[:space:]-]API|[Cc]onsole[[:space:]](command|script)'

validate_story_cache() {
    local story_id="$1" cache_path="$2" story_status="$3" cache_file
    cache_path="$(trim "$cache_path")"
    if [[ ! "$cache_path" =~ ^ui-story-runs/US-[0-9][0-9]+\.md$ ]]; then
        fail "$story_id has invalid run-cache path '$cache_path'"
        return
    fi
    cache_file="$plan_dir/$cache_path"
    if [ ! -f "$cache_file" ]; then
        fail "$story_id run cache is missing: $cache_file"
        return
    fi
    require_heading "$cache_file" "# Browser run cache: $story_id"
    require_heading "$cache_file" '## Starting state'
    require_heading "$cache_file" '## Buffered interaction sequence'
    require_heading "$cache_file" '## Waits and readiness'
    require_heading "$cache_file" '## Run result'
    grep -Eq "$interaction_pattern" "$cache_file" || fail "$story_id run cache has no direct UI input"
    if grep -Eq "$prohibited_pattern" "$cache_file"; then
        fail "$story_id run cache contains prohibited console, state, or direct-API input"
    fi
    if grep -Eq '<[^>]+>' "$cache_file"; then
        fail "$story_id run cache still contains a template placeholder"
    fi
    if [ "$complete_mode" = true ]; then
        if [ "$story_status" = '✅ passed' ]; then
            grep -Fqx -- '- Status: `✅ passed`' "$cache_file" || fail "$story_id run cache is not recorded as passed"
        elif [ "$story_status" = '⏭️ excluded' ]; then
            grep -Eq '^- Status: .*excluded' "$cache_file" || fail "$story_id excluded run cache is not recorded as excluded"
        fi
    fi
}

plan_validate_ui() {
    if [ "$ui_affected" = yes ]; then
        require_heading "$plan_dir/plan-description.md" '## UI validation'
        grep -Fqx -- '- Required: yes' "$plan_dir/plan-description.md" || fail "UI-affected plan must require UI validation"
        stories="$plan_dir/ui-user-stories.md"
        bugs_file="$plan_dir/bugs.md"
        if [ ! -f "$stories" ]; then
            fail "UI validation is required but ui-user-stories.md is missing"
        else
            grep -Eq '^# UI user stories: .+' "$stories" || fail "Missing user-story title in $stories"
            story_count=0
            # Cleared, not declared: plan_map_* maps are process-global, and this
            # function may run twice in one process.
            plan_map_clear bug_story_ids
            while IFS=$'\t' read -r story_id actions interaction status evidence related cache_path; do
                [ -n "$story_id" ] || continue
                story_count=$((story_count + 1))
                if ! [[ "$actions $interaction" =~ [Cc]lick|[Tt]ap|[Tt]ype|[Kk]eyboard|[Pp]ress|[Ss]wipe|[Pp]inch|[Dd]rag|[Ss]elect ]]; then
                    fail "$story_id has no documented direct user interaction"
                fi
                if [[ "$actions $interaction $evidence" =~ $prohibited_pattern ]]; then
                    fail "$story_id contains prohibited console, state, or direct-API test evidence"
                fi
                validate_story_cache "$story_id" "$cache_path" "$status"
                case "$status" in
                    '💤 untested'|'⏳ in progress'|'✅ passed'|'🐛 bug found'|'⏭️ excluded') ;;
                    *) fail "$story_id has an unsupported user-story status '$status'" ;;
                esac
                [ "$status" = '🐛 bug found' ] && plan_map_set bug_story_ids "$story_id" 1
                verification_count=0
                while IFS= read -r related_id; do
                    [ -n "$related_id" ] || continue
                    if ! plan_map_has unit_type "$related_id"; then
                        fail "$story_id refers to unknown work unit $related_id"
                    elif plan_map_load unit_type "$related_id" && [ "$plan_map_value" = verification ]; then
                        verification_count=$((verification_count + 1))
                    fi
                done < <(printf '%s\n' "$related" | grep -oE 'W[0-9][0-9]+' || true)
                if [ "$verification_count" -eq 0 ]; then
                    fail "$story_id has no related verification work unit"
                fi
                if [ "$complete_mode" = true ]; then
                    case "$status" in
                        '✅ passed') ;;
                        '⏭️ excluded')
                            [[ "$evidence" =~ [Uu]ser[[:space:]-]approved ]] || fail "$story_id is excluded without recorded user approval"
                            ;;
                        *) fail "$story_id is not validated at plan completion ($status)" ;;
                    esac
                fi
            done < <(
                awk -F'|' '
                    /^\|[[:space:]]*US-[0-9][0-9]+[[:space:]]*\|/ {
                        for (i = 2; i <= 10; i++) {
                            if (i != 2 && i != 4 && i != 5 && i != 7 && i != 8 && i != 9 && i != 10) continue
                            value = $i
                            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
                            printf "%s%s", value, (i == 10 ? ORS : "\t")
                        }
                    }
                ' "$stories"
            )
            if [ "$story_count" -eq 0 ]; then
                fail "ui-user-stories.md has no story rows; use IDs such as US-01"
            fi
        fi
        if [ ! -f "$bugs_file" ]; then
            fail "UI validation requires bugs.md"
        else
            grep -Eq '^# UI bugs: .+' "$bugs_file" || fail "Missing UI bug title in $bugs_file"
            while IFS= read -r story_id; do
                [ -n "$story_id" ] || continue
                grep -Eq "^\\|[[:space:]]*BUG-[0-9]+[[:space:]]*\\|[[:space:]]*${story_id}[[:space:]]*\\|.*-investigate-.*\\|.*-fix-" "$bugs_file" || fail "$story_id bug lacks linked investigation and fix goals"
            done < <(plan_map_keys bug_story_ids)
            if [ "$complete_mode" = true ] && grep -Eq '^\|[[:space:]]*BUG-[0-9]+[[:space:]]*\|.*\|[[:space:]]*(💤 open|⏳ open|⏳ in progress)[[:space:]]*\|$' "$bugs_file"; then
                fail "UI bugs.md has unresolved bugs at plan completion"
            fi
        fi
    elif grep -Fqx -- '- Required: yes' "$plan_dir/plan-description.md"; then
        fail "Plan requires UI validation but declares UI affected: no"
    fi
}
