#!/usr/bin/env bash
set -euo pipefail

complete_mode=false
if [ "$#" -eq 2 ] && [ "$1" = '--complete' ]; then
    complete_mode=true
    plan_dir="$2"
elif [ "$#" -eq 1 ]; then
    plan_dir="$1"
else
    echo "Usage: $(basename "$0") [--complete] <plan-directory>" >&2
    exit 64
fi

inventory="$plan_dir/work-unit-inventory.md"
errors=0

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    errors=$((errors + 1))
}

trim() {
    local value="$1"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    value="${value#\`}"; value="${value%\`}"
    printf '%s' "$value"
}

require_heading() {
    local file="$1" heading="$2"
    grep -Fqx "$heading" "$file" || fail "Missing '$heading' in $file"
}

field_value=''

get_single_field() {
    local file="$1" label="$2"
    local count value
    count="$(grep -Ec "^[[:space:]]*-[[:space:]]*${label}:[[:space:]]*.+[[:space:]]*$" "$file" || true)"
    if [ "$count" -ne 1 ]; then
        fail "$file must declare exactly one '$label:' field (found $count)"
        field_value=''
        return
    fi
    value="$(sed -nE "s/^[[:space:]]*-[[:space:]]*${label}:[[:space:]]*(.*[^[:space:]])[[:space:]]*$/\1/p" "$file")"
    field_value="$(trim "$value")"
}

if [ ! -d "$plan_dir" ]; then
    echo "Plan directory not found: $plan_dir" >&2
    exit 66
fi
if [ ! -f "$plan_dir/plan-description.md" ]; then
    fail "Missing plan-description.md"
fi
if [ ! -f "$inventory" ]; then
    fail "Missing work-unit-inventory.md"
fi
if [ "$errors" -gt 0 ]; then
    exit 1
fi

for heading in \
    '## Current state' \
    '## Desired outcome' \
    '## Approach' \
    '## Scope' \
    '## Affected areas' \
    '## Constraints and decisions' \
    '## Risks and open questions' \
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
    grep -Fqx -- '- Status: `✅ approved`' "$review_file" || fail "Adversarial review is not approved"
    grep -Fqx -- '- Status: ✅ approved' "$plan_dir/plan-description.md" || fail "Plan description does not mirror approved adversarial-review status"
    if grep -Eq '^\|[[:space:]]*AR-[0-9]+[[:space:]]*\|.*\|[[:space:]]*(💤 open|⏳ in progress)[[:space:]]*\|$' "$review_file"; then
        fail "Adversarial review has unresolved findings"
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

declare -A unit_type unit_file unit_scope unit_subscope unit_goal unit_step unit_depends seen_steps goal_units coverage_ids
unit_ids=()

while IFS=$'\t' read -r id type file scope subscope intended depends goal step; do
    [ -n "$id" ] || continue
    if [[ ! "$id" =~ ^W[0-9][0-9]+$ ]]; then
        fail "Invalid work-unit ID: $id"
        continue
    fi
    if [ -n "${unit_type[$id]+x}" ]; then
        fail "Duplicate work-unit ID: $id"
        continue
    fi
    case "$type" in
        source|markup|style|test|config|docs|data|generated|discovery|verification) ;;
        *) fail "$id has unsupported type '$type'" ;;
    esac
    if [ -z "$file" ] || [ -z "$scope" ] || [ -z "$subscope" ] || [ -z "$intended" ] || [ -z "$goal" ] || [ -z "$step" ]; then
        fail "$id has an empty required work-unit field"
    fi
    if [ "$type" = verification ] && [ "$file" != N/A ]; then
        fail "$id is verification and must use File 'N/A'"
    fi
    if [ "$type" != verification ] && [ "$file" = N/A ]; then
        fail "$id is not verification and must name one file"
    fi
    if [[ "$file" == *'*'* || "$file" == */ ]]; then
        fail "$id must name one concrete file, not a glob or directory: $file"
    fi
    if [ "$type" != verification ] && { [[ "$scope" == *','* ]] || [[ "$scope" == *' and '* ]]; }; then
        fail "$id lists multiple symbols or scopes: $scope"
    fi
    if [ "$type" = style ] && [[ ! "$scope" =~ ^[.#][A-Za-z_-][A-Za-z0-9_-]*$ ]]; then
        fail "$id style scope must be one CSS selector, such as .completion-message"
    fi
    if [ "$type" = markup ] && [[ ! "$scope" =~ ^[#.][A-Za-z_-][A-Za-z0-9_-]*$ ]]; then
        fail "$id markup scope must be one named DOM selector, such as #checkout-summary"
    fi
    if [[ ! "$goal" =~ ^[0-9][0-9]-[a-z0-9-]+$ ]]; then
        fail "$id has invalid goal name '$goal'"
    fi
    if [[ ! "$step" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]]; then
        fail "$id has invalid step name '$step'"
    fi
    if [ "$subscope" != N/A ] && { [[ "$subscope" == *','* ]] || [[ "$subscope" == *' and '* ]]; }; then
        fail "$id lists multiple subscope targets: $subscope"
    fi
    unit_type[$id]="$type"; unit_file[$id]="$file"; unit_scope[$id]="$scope"; unit_subscope[$id]="$subscope"
    unit_goal[$id]="$goal"; unit_step[$id]="$step"; unit_ids+=("$id")
    unit_depends[$id]="$depends"
    if [ -n "${seen_steps[$goal/$step]+x}" ]; then
        fail "Multiple work units are assigned to $goal/steps/$step.md"
    fi
    seen_steps[$goal/$step]="$id"
    goal_units[$goal]="${goal_units[$goal]:-} $id"
done < <(
    awk -F'|' '
        /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
            for (i = 2; i <= 10; i++) {
                value = $i
                gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
                gsub(/^`|`$/, "", value)
                printf "%s%s", value, (i == 10 ? ORS : "\t")
            }
        }
    ' "$inventory"
)

if [ "${#unit_ids[@]}" -eq 0 ]; then
    fail "No work-unit rows found; use IDs such as W01 in the Work units table"
fi

while IFS= read -r coverage_id; do
    [ -n "$coverage_id" ] && coverage_ids[$coverage_id]=1
done < <(
    awk -F'|' '
        /^## Work units/ { exit }
        /^\|/ && $2 !~ /Required outcome/ && $2 !~ /^-+$/ {
            print $3
        }
    ' "$inventory" | grep -oE 'W[0-9][0-9]+' || true
)
if [ "${#coverage_ids[@]}" -eq 0 ]; then
    fail "Definition-of-done coverage has no work-unit references"
fi
for id in "${unit_ids[@]}"; do
    [ -n "${coverage_ids[$id]+x}" ] || fail "$id is not linked to a definition-of-done item"
done
for coverage_id in "${!coverage_ids[@]}"; do
    [ -n "${unit_type[$coverage_id]+x}" ] || fail "Definition-of-done coverage names unknown work unit $coverage_id"
done

declare -A visit_state
check_dependencies() {
    local id="$1" dependency
    case "${visit_state[$id]:-}" in
        visiting) fail "Dependency cycle includes $id"; return ;;
        done) return ;;
    esac
    visit_state[$id]=visiting
    while IFS= read -r dependency; do
        [ -n "$dependency" ] || continue
        if [ -z "${unit_type[$dependency]+x}" ]; then
            fail "$id depends on unknown work unit $dependency"
        else
            check_dependencies "$dependency"
        fi
    done < <(printf '%s\n' "${unit_depends[$id]}" | grep -oE 'W[0-9][0-9]+' || true)
    visit_state[$id]=done
}
for id in "${unit_ids[@]}"; do
    check_dependencies "$id"
done

declare -A proof_seen
depends_on() {
    local candidate="$1" required="$2" dependency key
    [ "$candidate" = "$required" ] && return 0
    key="$candidate/$required"
    [ -n "${proof_seen[$key]+x}" ] && return 1
    proof_seen[$key]=1
    while IFS= read -r dependency; do
        [ -n "$dependency" ] || continue
        [ "$dependency" = "$required" ] && return 0
        depends_on "$dependency" "$required" && return 0
    done < <(printf '%s\n' "${unit_depends[$candidate]}" | grep -oE 'W[0-9][0-9]+' || true)
    return 1
}

for id in "${unit_ids[@]}"; do
    case "${unit_type[$id]}" in
        source|markup|style|config|data|generated)
            has_proof=false
            for proof_id in "${unit_ids[@]}"; do
                case "${unit_type[$proof_id]}" in
                    test|verification)
                        proof_seen=()
                        if depends_on "$proof_id" "$id"; then
                            has_proof=true
                            break
                        fi
                        ;;
                esac
            done
            [ "$has_proof" = true ] || fail "$id has no downstream test or verification work unit"
            ;;
    esac
done

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
        declare -A bug_story_ids
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
            [ "$status" = '🐛 bug found' ] && bug_story_ids[$story_id]=1
            verification_count=0
            while IFS= read -r related_id; do
                [ -n "$related_id" ] || continue
                if [ -z "${unit_type[$related_id]+x}" ]; then
                    fail "$story_id refers to unknown work unit $related_id"
                elif [ "${unit_type[$related_id]}" = verification ]; then
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
        for story_id in "${!bug_story_ids[@]}"; do
            grep -Eq "^\\|[[:space:]]*BUG-[0-9]+[[:space:]]*\\|[[:space:]]*${story_id}[[:space:]]*\\|.*-investigate-.*\\|.*-fix-" "$bugs_file" || fail "$story_id bug lacks linked investigation and fix goals"
        done
        if [ "$complete_mode" = true ] && grep -Eq '^\|[[:space:]]*BUG-[0-9]+[[:space:]]*\|.*\|[[:space:]]*(💤 open|⏳ open|⏳ in progress)[[:space:]]*\|$' "$bugs_file"; then
            fail "UI bugs.md has unresolved bugs at plan completion"
        fi
    fi
elif grep -Fqx -- '- Required: yes' "$plan_dir/plan-description.md"; then
    fail "Plan requires UI validation but declares UI affected: no"
fi

for goal_dir in "$plan_dir"/[0-9][0-9]-*/; do
    [ -d "$goal_dir" ] || continue
    goal_name="$(basename "$goal_dir")"
    goal_file="$goal_dir/goal.md"
    if [ ! -f "$goal_file" ]; then
        fail "Missing goal.md for $goal_name"
        continue
    fi
    for heading in \
        '## Current state and prior-goal handoffs' \
        '## Outcome and definition of done' \
        '## Why this goal is needed' \
        '## Scope' \
        '## Affected files, systems, data, and interfaces' \
        '## Dependencies and handoffs' \
        '## Implementation approach, risks, and edge cases' \
        '## Owned work units'; do
        require_heading "$goal_file" "$heading"
    done
    count=0
    for id in ${goal_units[$goal_name]:-}; do
        count=$((count + 1))
        grep -Fq "$id" "$goal_file" || fail "$goal_file does not name assigned work unit $id"
    done
    if [ "$count" -eq 0 ]; then
        fail "$goal_name has no assigned work units"
    fi
    if [ "$count" -eq 1 ]; then
        only_id="${goal_units[$goal_name]# }"
        case "${unit_type[$only_id]}" in
            docs|config|discovery|verification) ;;
            *) fail "$goal_name has one ${unit_type[$only_id]} work unit; add its test/proof or merge it into its demonstrable outcome" ;;
        esac
        require_heading "$goal_file" '## Goal-size exception'
    elif [ "$count" -gt 10 ]; then
        fail "$goal_name has $count work units; split it at a stable outcome boundary"
    fi
done

for id in "${unit_ids[@]}"; do
    goal_dir="$plan_dir/${unit_goal[$id]}"
    step_file="$goal_dir/steps/${unit_step[$id]}.md"
    if [ ! -d "$goal_dir" ]; then
        fail "$id references missing goal directory ${unit_goal[$id]}"
        continue
    fi
    if [ ! -f "$step_file" ]; then
        fail "$id references missing step file $step_file"
        continue
    fi
    require_heading "$step_file" '## Ownership'
    require_heading "$step_file" '## Change target'
    require_heading "$step_file" '## Objective'
    require_heading "$step_file" '## Instructions'
    require_heading "$step_file" '## Acceptance criteria'
    require_heading "$step_file" '## Handoff'
    require_heading "$step_file" '## Atomicity check'
    grep -Fqx -- "- Goal: \`${unit_goal[$id]}\`" "$step_file" || fail "$step_file has wrong owning goal for $id"
    grep -Fqx -- "- Work unit: \`$id\`" "$step_file" || fail "$step_file has wrong work-unit ID"
    grep -Fqx -- "- Type: \`${unit_type[$id]}\`" "$step_file" || fail "$step_file has wrong type for $id"
    get_single_field "$step_file" 'File'; actual_file="$field_value"
    get_single_field "$step_file" 'Primary symbol or file scope'; actual_scope="$field_value"
    get_single_field "$step_file" 'Subscope'; actual_subscope="$field_value"
    [ "$actual_file" = "${unit_file[$id]}" ] || fail "$step_file file does not match $id inventory row"
    [ "$actual_scope" = "${unit_scope[$id]}" ] || fail "$step_file primary scope does not match $id inventory row"
    [ "$actual_subscope" = "${unit_subscope[$id]}" ] || fail "$step_file subscope does not match $id inventory row"
    grep -Fqx -- '- [x] This step owns exactly one inventory work unit.' "$step_file" || fail "$step_file has not confirmed one work unit"
    grep -Fqx -- '- [x] No other file, symbol, test target, or verification flow changes here.' "$step_file" || fail "$step_file has not confirmed target isolation"
    grep -Fqx -- '- [x] Any follow-on target has a separately named work unit and step.' "$step_file" || fail "$step_file has not confirmed follow-on ownership"
done

for goal_dir in "$plan_dir"/[0-9][0-9]-*/; do
    [ -d "$goal_dir/steps" ] || continue
    goal_name="$(basename "$goal_dir")"
    while IFS= read -r step_file; do
        step_name="$(basename "$step_file" .md)"
        get_single_field "$step_file" 'Work unit'; declared_id="$field_value"
        if [ -z "${unit_type[$declared_id]+x}" ]; then
            fail "$step_file declares unlisted work unit '$declared_id'"
        elif [ "${unit_goal[$declared_id]}" != "$goal_name" ] || [ "${unit_step[$declared_id]}" != "$step_name" ]; then
            fail "$step_file does not match the inventory assignment for $declared_id"
        fi
    done < <(find "$goal_dir/steps" -maxdepth 1 -type f -name '[0-9][0-9]-step-*.md' ! -name '*-testing.md' | sort)
    while IFS= read -r step_file; do
        step_name="$(basename "$step_file")"
        if [[ ! "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+\.md$ ]]; then
            fail "$step_file is not a numbered step file"
        fi
    done < <(find "$goal_dir/steps" -maxdepth 1 -type f -name '*.md' ! -name '*-testing.md' | sort)
done

if [ "$complete_mode" = true ]; then
    plan_progress="$plan_dir/progress.md"
    if [ ! -f "$plan_progress" ]; then
        fail "Completion requires plan-level progress.md"
    else
        for goal_name in "${!goal_units[@]}"; do
            goal_status="$(awk -F'|' -v wanted="$goal_name" '
                /^\|/ { key=$2; status=$4; gsub(/^[[:space:]]+|[[:space:]]+$/, "", key); gsub(/^[[:space:]]+|[[:space:]]+$/, "", status); if (key == wanted) print status }
            ' "$plan_progress")"
            [ "$goal_status" = '✅ completed' ] || fail "$goal_name is not completed in plan progress"
        done
    fi
    for id in "${unit_ids[@]}"; do
        goal_progress="$plan_dir/${unit_goal[$id]}/progress.md"
        if [ ! -f "$goal_progress" ]; then
            fail "$id completion requires $goal_progress"
            continue
        fi
        step_status="$(awk -F'|' -v wanted="${unit_step[$id]}" '
            /^\|/ { key=$3; status=$5; gsub(/^[[:space:]]+|[[:space:]]+$/, "", key); gsub(/^[[:space:]]+|[[:space:]]+$/, "", status); if (key == wanted) print status }
        ' "$goal_progress")"
        [ "$step_status" = '✅ completed' ] || fail "$id is not completed in ${unit_goal[$id]} progress"
    done
fi

if [ "$errors" -gt 0 ]; then
    printf 'Plan validation failed with %d error(s).\n' "$errors" >&2
    exit 1
fi

printf 'Plan validation passed: %d work units across %d goals.\n' \
    "${#unit_ids[@]}" "${#goal_units[@]}"
