#!/usr/bin/env bash
set -euo pipefail

case "${1:-}" in
    -h|--help)
        echo "Usage: $(basename "$0") [--complete] [--propagation|--no-propagation] [--stale <file-of-phrases>|default] <plan-directory>" >&2
        exit 0
        ;;
esac

complete_mode=false
propagation_mode=true
stale_file=""
stale_only=false
stale_requested=false
filtered_args=()
for arg in "$@"; do
    case "$arg" in
        --complete) complete_mode=true ;;
        --propagation) propagation_mode=true ;;
        --no-propagation) propagation_mode=false ;;
        --stale)
            stale_only=true
            stale_requested=true
            ;;
        --stale=*)
            stale_file="${arg#--stale=}"
            stale_requested=true
            ;;
        --)
            filtered_args+=("$arg")
            ;;
        *)
            if [ "$stale_only" = true ]; then
                stale_file="$arg"
                stale_only=false
            else
                filtered_args+=("$arg")
            fi
            ;;
    esac
done
set -- "${filtered_args[@]}"

if [ "$#" -eq 1 ]; then
    plan_dir="$1"
elif [ "$#" -eq 2 ] && [ "$1" = '--complete' ]; then
    complete_mode=true
    plan_dir="$2"
else
    echo "Usage: $(basename "$0") [--complete] [--propagation] [--stale <file-of-phrases>|default] <plan-directory>" >&2
    exit 64
fi

inventory="$plan_dir/work-unit-inventory.md"
errors=0

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    errors=$((errors + 1))
}

warn() {
    printf 'WARN: %s\n' "$*" >&2
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

# Template-placeholder check (report 15 §4, report 16): detection is LITERAL
# registry membership. planning/placeholders.json is the exact, finite list of
# <...> tokens the skill's templates emit. A token is a placeholder iff it is
# registered here — no shape heuristic, so single-word (<why>), spaced, and
# hyphenated tokens are all handled by construction, and author-written <...>
# prose/patterns are never flagged because they are not in the list (no
# code-span exemption needed). Fenced code blocks are still skipped. Each entry
# carries a surface (authored | generated) that drives the verdict:
#   - not registered              -> ignored (author's own prose)
#   - registered, authored doc    -> WARN; FAIL under --complete
#   - registered, generated doc   -> FAIL always (no human ever fills it)
registry_file="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/placeholders.json"
placeholder_re='<[^<>]*[A-Za-z][^<>]*>'

# Print the registry surface for a token; empty if not registered.
token_surface() {
    [ -f "$registry_file" ] || return 1
    jq -r --arg t "$1" '.placeholders[] | select(.token == $t) | .surface' "$registry_file" 2>/dev/null
}

# Scan one file for registered placeholders and apply the surface-based verdict.
#   check_placeholders <authored|generated> <label> <file>
check_placeholders() {
    local surface_arg="$1" label="$2" file="$3" tok surf
    [ -f "$file" ] || return 0
    while IFS= read -r tok; do
        surf="$(token_surface "$tok")"
        [ -n "$surf" ] || continue   # not registered -> author prose, ignore
        if [ "$surface_arg" = generated ]; then
            fail "$label contains a registered placeholder that no author will fill: $tok"
        elif [ "$complete_mode" = true ]; then
            fail "$label still contains a registered placeholder: $tok"
        else
            warn "$label contains a registered placeholder (fill before completion): $tok"
        fi
    done < <(awk -v re="$placeholder_re" '
        /^```/ { in_fence = !in_fence; next }
        !in_fence {
            while (match($0, re)) {
                print substr($0, RSTART, RLENGTH)
                $0 = substr($0, RSTART + RLENGTH)
            }
        }
    ' "$file" | sort -u)
}
# Authored documents: plan narrative, review, inventory, goals, and steps.
for doc in "${plan_docs[@]}"; do
    [ -f "$doc" ] || continue
    check_placeholders authored "$(basename "$doc")" "$doc"
done
# Generated artifacts: plan progress tracker.
[ -f "$plan_dir/progress.md" ] && check_placeholders generated "plan progress.md" "$plan_dir/progress.md"
# Generated artifacts: goal progress trackers and goal-size-exception sections.
for goal_dir in "$plan_dir"/[0-9][0-9]-*/; do
    [ -d "$goal_dir" ] || continue
    [ -f "$goal_dir/progress.md" ] && check_placeholders generated "$(basename "$goal_dir") progress.md" "$goal_dir/progress.md"
    [ -f "$goal_dir/goal.md" ] || continue
    while IFS= read -r tok; do
        surf="$(token_surface "$tok")"
        [ -n "$surf" ] || continue
        fail "$(basename "$goal_dir") goal-size exception contains a registered placeholder that no author will fill: $tok"
    done < <(awk -v re="$placeholder_re" '
        /^## Goal-size exception/ { in_sec = 1; next }
        in_sec && /^## / && !/^## Goal-size exception/ { exit }
        in_sec && /^```/ { in_fence = !in_fence; next }
        !in_fence && in_sec {
            while (match($0, re)) {
                print substr($0, RSTART, RLENGTH)
                $0 = substr($0, RSTART + RLENGTH)
            }
        }
    ' "$goal_dir/goal.md" | sort -u)
done
# Generated artifacts: UI-story run caches.
if [ -d "$plan_dir/ui-story-runs" ]; then
    while IFS= read -r cache_file; do
        check_placeholders generated "$(basename "$cache_file") run cache" "$cache_file"
    done < <(find "$plan_dir/ui-story-runs" -maxdepth 1 -name '*.md' -type f 2>/dev/null)
fi

# Default stale phrase list (report 8): case-count wording is the anti-pattern.
# A count drifts the moment a case is added, so "every case enumerated in the
# instructions" is the drift-proof form; these phrases flag the countable form
# so a fix converts "all four states" into the explicit enumeration. A
# paragraph that also records a history marker is exempt (it is a deliberate
# corrective reference, not live text). Pass --stale with a file to extend, or
# --stale default to run this list alone.
stale_default_phrases=(
    'all four'
    'all six'
    'all three'
    'all five'
    'the eleven'
    'the six states'
    'all states'
    'four per-state'
    # Class B (report 12): a criterion demanding an identical/byte-identical
    # output the target cannot produce (embedded metadata, live dates) is a
    # gate that fails correct work. Flag the wording so it is revisited.
    'byte-identical'
    'byte-for-byte'
    'pixel-identical'
    'exactly identical'
)
# --- stale-wording sweep (--stale <file-of-phrases>): a listed phrase that
#     still appears in a paragraph that does not also record a history marker
#     (e.g. "an earlier version", "previously") is stale — a fix that removed
#     the phrase from one paragraph but left another unmarked is half-landed.
#     Markers are multi-word historical signals ONLY: a single adjective such
#     as "legacy" or "obsolete" may legitimately be part of the stale phrase
#     itself, so it would mask a match. Deliberate references to corrected
#     history pass while an unfixed sibling fails.
stale_markers='an earlier version|previously|superseded by|supersedes|no longer|was removed|historically|now replaced by'
# The stale sweep scans the same documents as find --in all, INCLUDING the
# *-testing.md companions — the surface where case-count defects live and the
# one most likely to drift (report 5 §1 / report 8). plan_docs above excludes
# companions for the structural hardening checks, so build a separate list here.
stale_docs=("${plan_docs[@]}")
for step_file in "$plan_dir"/*/steps/*-testing.md; do
    [ -f "$step_file" ] || continue
    stale_docs+=("$step_file")
done
stale_scan_doc() {
    local file="$1" phrase="$2"
    awk -v phrase="$phrase" -v markers="$stale_markers" '
        function flush() {
            if (label != "" && index(content, phrase) > 0 && content !~ markers) {
                printf "%s: %s\n", FILENAME, label
            }
            label = ""; content = ""
        }
        /^#+ / { flush(); label = $0; next }
        /^[[:space:]]*$/ { next }
        { if (label != "") content = content $0 "\n" }
        END { flush() }
    ' "$file"
}
if [ "$stale_requested" = true ]; then
    # --stale default (or --stale with no file) uses the bundled case-count
    # phrase list; --stale <file> uses that file. When a file is given, it is
    # used alone (extend it by adding phrases to the file).
    if [ -n "$stale_file" ] && [ "$stale_file" != "default" ]; then
        if [ ! -f "$stale_file" ]; then
            fail "--stale file not found: $stale_file"
        fi
        stale_phrases_file="$stale_file"
    else
        stale_phrases_file="$(mktemp "${TMPDIR:-/tmp}/plan-stale-default.XXXXXX")"
        trap 'rm -f "$stale_phrases_file"' EXIT
        printf '%s\n' "${stale_default_phrases[@]}" > "$stale_phrases_file"
    fi
    while IFS= read -r phrase; do
        [ -n "${phrase//[[:space:]]/}" ] || continue
        for doc in "${stale_docs[@]}"; do
            [ -f "$doc" ] || continue
            hits="$(stale_scan_doc "$doc" "$phrase")"
            if [ -n "$hits" ]; then
                fail "stale phrase '$phrase' appears in an unmarked paragraph: $(printf '%s' "$hits" | tr '\n' ' ')"
            fi
        done
    done < "$stale_phrases_file"
    if [ -z "$stale_file" ] || [ "$stale_file" = "default" ]; then
        rm -f "$stale_phrases_file"
        trap - EXIT
    fi
fi

declare -A unit_type unit_file unit_scope unit_subscope unit_goal unit_step unit_depends seen_steps goal_units goal_testing_required
declare -A coverage_ids=()
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
    # A scope names one symbol. Key on the count of ::-qualified symbols, not
    # on conjunctions: "X::m() and the EmployeeSet value object, both new in
    # this file" is a legitimate single-file description that a plain ' and '
    # check would wrongly reject. A comma list still signals multiple scopes.
    if [ "$type" != verification ]; then
        sym_count="$(printf '%s' "$scope" | grep -oE '[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*(\(\))?' | wc -l | tr -d ' ')" || true
        if [ "$sym_count" -gt 1 ]; then
            fail "$id lists multiple symbols or scopes: $scope"
        elif [[ "$scope" == *','* ]]; then
            fail "$id lists multiple symbols or scopes: $scope"
        fi
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
            [ "${goal_testing_required[${unit_goal[$id]}]:-}" = yes ] || continue
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

validate_goal_testing_requirement() {
    local goal_name="$1" goal_file="$2" row required rationale
    require_heading "$goal_file" '## Testing requirement'
    row="$(awk -F'|' '
        $0 == "## Testing requirement" { in_section = 1; next }
        in_section && /^## / { in_section = 0 }
        in_section && $0 == "| Test required | Rationale |" { headers++ }
        in_section && $0 == "|---|---|" { separators++ }
        in_section && /^\|[[:space:]]*(yes|no)[[:space:]]*\|[^|]+\|$/ {
            rows++
            value = $2; reason = $3
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", reason)
        }
        END {
            if (headers != 1 || separators != 1 || rows != 1) exit 2
            printf "%s\t%s\n", value, reason
        }
    ' "$goal_file")" || {
        fail "$goal_file must contain exactly one Test required/Rationale table row"
        goal_testing_required[$goal_name]=''
        return
    }
    IFS=$'\t' read -r required rationale <<< "$row"
    if [ -z "$rationale" ] || [[ "$rationale" == *'<'*'>'* ]]; then
        fail "$goal_file must explain why testing is or is not required"
    fi
    goal_testing_required[$goal_name]="$required"
}

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
        '## Owned work units' \
        '## Testing requirement'; do
        require_heading "$goal_file" "$heading"
    done
    validate_goal_testing_requirement "$goal_name" "$goal_file"
    if grep -Fq '<required only when this goal has one permitted work unit>' "$goal_file"; then
        fail "$goal_name has an unfilled goal-size placeholder; fill the reason or remove the section"
    fi
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
    test_units=0
    for id in ${goal_units[$goal_name]:-}; do
        case "${unit_type[$id]}" in
            test|verification) test_units=$((test_units + 1)) ;;
        esac
    done
    if [ "${goal_testing_required[$goal_name]:-}" = yes ] && [ "$test_units" -eq 0 ]; then
        fail "$goal_name declares testing is required but has no test or verification work unit"
    elif [ "$test_units" -gt 0 ] && [ "${goal_testing_required[$goal_name]:-}" != yes ]; then
        fail "$goal_name has a test or verification work unit but its testing requirement is not yes"
    fi
    if [ "${goal_testing_required[$goal_name]:-}" = yes ]; then
        for id in ${goal_units[$goal_name]:-}; do
            [ "${unit_type[$id]}" = docs ] && continue
            companion="$plan_dir/$goal_name/steps/${unit_step[$id]}-testing.md"
            [ -f "$companion" ] || fail "$id requires testing instructions at $companion"
        done
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

# --- propagation checks (--propagation): the surfaces of a work unit must
#     agree. A finding cites one surface; a fix must reach the others, and this
#     is the mechanical part of that contract. ---
if [ "$propagation_mode" = true ]; then
    # (a) Naming a class in instructions does not schedule an EDIT to it.
    #     Flag a ::-symbol or path on an edit-intent line when ALL hold:
    #       1. its namespace root or path prefix is one the plan itself edits
    #          (derived from the inventory's File column — e.g. BigBridge\,
    #          Proforto\, app/code/, app/design/), which excludes vendor seams
    #          (Magento\, Amasty\, Magestore_, a vendor/ path) by construction,
    #          because a plan never lists a vendor path as a change target;
    #       2. no inventory row owns it;
    #       3. the naming line instructs an edit, not a boundary statement.
    #     This catches "instruct an edit to a project symbol no unit owns"
    #     (the RequestEmployeeSet class three plans consumed but none built)
    #     without a hardcoded vendor list.
    declare -a project_prefixes=()
    for candidate in "${unit_ids[@]}"; do
        fc="${unit_file[$candidate]}"
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
    for id in "${unit_ids[@]}"; do
        step_file="$plan_dir/${unit_goal[$id]}/steps/${unit_step[$id]}.md"
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
        # Well-formed Class::method tokens only. Exclude X::class (a PHP class
        # constant, not a method) and Vendor_Module::path template identifiers
        # (which the tokeniser would otherwise truncate into a plausible but
        # nonexistent Class::method).
        tokens="$(printf '%s' "$edit_lines" | grep -oE '\b[A-Z][A-Za-z0-9_]*(\\[A-Za-z_][A-Za-z0-9_]*)*::[A-Za-z_][A-Za-z0-9_]*\(?' | sed -E 's/\($//' | sort -u || true)"
        for token in $tokens; do
            # Skip X::class — a PHP class constant (Foo::class, Bar::class),
            # not a method call, and not an edit target.
            [ "${token##*::}" = "class" ] && continue
            # Template-identifier guard: Vendor_Module::<path> (Magento_Weee::
            # email/items/price/row.phtml) is a template id, not a class method.
            if [[ "$token" =~ ^[A-Z][a-zA-Z0-9]*_[A-Z][a-zA-Z0-9]*:: ]]; then
                # Confirm the full source line carries a path after ::(a slash).
                if printf '%s' "$edit_lines" | grep -qE "${token%%::*}[A-Za-z0-9_]*::[^ (]*/"; then
                    continue
                fi
            fi
            # Condition 1: the symbol's namespace root must be a project prefix
            # the plan edits (vendor seams drop out here by construction).
            klass="${token%%::*}"
            klass_short="${klass##*\\}"
            prefix_match=false
            for prefix in "${project_prefixes[@]}"; do
                case "$klass" in
                    "$prefix"*|"$klass_short") prefix_match=true; break ;;
                esac
                [[ "$klass_short" == "$prefix"* ]] && { prefix_match=true; break; }
            done
            [ "$prefix_match" = true ] || continue
            # Condition 2: no inventory row owns it (file basename or scope).
            owned=false
            for candidate in "${unit_ids[@]}"; do
                file_cell="${unit_file[$candidate]}"
                scope_cell="${unit_scope[$candidate]}"
                scope_class="${scope_cell%%::*}"
                scope_class_short="${scope_class##*\\}"
                [ "$(basename "$file_cell" 2>/dev/null)" = "$klass_short" ] && { owned=true; break; }
                [ "$file_cell" = "$klass" ] && { owned=true; break; }
                [ "$scope_class" = "$klass" ] && { owned=true; break; }
                [ "$scope_class_short" = "$klass_short" ] && { owned=true; break; }
            done
            # This rule cannot distinguish "edit this" from "this is where we
            # attach" from text alone (the short class form carries no
            # namespace, so the project-prefix gate cannot help). It is a
            # heuristic: surface it as a WARN for a human to skim, never a
            # blocking FAIL — 17 warnings read fine, 17 FAILs block the plan.
            if [ "$owned" = false ] && [ "$token" != "$id" ]; then
                warn "$id instructions mention '$token' which no inventory row owns; verify it is a seam description, or add a discovery/ownership row if it is an edit target"
            fi
        done
        # (b) Removed by report 7: cross-mention warnings fired on any passing
        #     sibling reference ("W83 owns this payload, do not duplicate it")
        #     and produced 500+ warnings that penalised the seven-surface prose.
    done

    # (c) A verification unit must be able to reach (transitively) every same-
    #     goal unit it grades. Reverse edges are deliberate (a baseline-capture
    #     verification runs FIRST by definition), so an edge in the opposite
    #     direction is a guard, not a violation. Transitive ordering satisfies
    #     the rule without a direct edge.
    dep_reaches() {
        local from="$1" to="$2" dep key
        [ "$from" = "$to" ] && return 0
        key="$from/$to"
        [ -n "${dep_seen[$key]+x}" ] && return 1
        dep_seen[$key]=1
        while IFS= read -r dep; do
            [ -n "$dep" ] || continue
            [ "$dep" = "$to" ] && return 0
            dep_reaches "$dep" "$to" && return 0
        done < <(printf '%s' "${unit_depends[$from]}" | grep -oE 'W[0-9][0-9]+' || true)
        return 1
    }
    declare -A dep_seen=()
    for id in "${unit_ids[@]}"; do
        [ "${unit_type[$id]}" = verification ] || continue
        step_file="$plan_dir/${unit_goal[$id]}/steps/${unit_step[$id]}.md"
        [ -f "$step_file" ] || continue
        named_units="$(grep -oE '\bW[0-9][0-9]+\b' "$step_file" | sort -u)" || true
        for named in $named_units; do
            [ "$named" = "$id" ] && continue
            # A WNN not in this plan's inventory is a cross-plan reference
            # ("the extended-rendering plan's W04"), correct prose, not a typo.
            if [ -z "${unit_type[$named]+x}" ]; then
                continue
            fi
            if [ "${unit_goal[$named]}" = "${unit_goal[$id]}" ]; then
                dep_seen=()
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

    # (c2) A testing companion must not reference work units the step does not
    #      own or depend on: the executor runs the companion, so a stale unit
    #      reference there (a test that moved, a file owned by another unit)
    #      directs execution at the wrong target.
    for id in "${unit_ids[@]}"; do
        companion="$plan_dir/${unit_goal[$id]}/steps/${unit_step[$id]}-testing.md"
        [ -f "$companion" ] || continue
        deps="$(printf '%s' "${unit_depends[$id]}" | grep -oE 'W[0-9][0-9]+' | sort -u | tr '\n' ' ')" || true
        named_units="$(grep -oE '\bW[0-9][0-9]+\b' "$companion" | sort -u)" || true
        for named in $named_units; do
            [ "$named" = "$id" ] && continue
            # Cross-plan reference, not a typo: a WNN outside this plan is prose.
            if [ -z "${unit_type[$named]+x}" ]; then
                continue
            fi
            # A companion may correctly reference a same-goal test/verification
            # unit ("automated tests: covered by WNN") — that is proof-coverage
            # prose, not a dependency claim (report 14 §5). Skip it.
            if [ "${unit_goal[$named]}" = "${unit_goal[$id]}" ] && \
               { [ "${unit_type[$named]}" = test ] || [ "${unit_type[$named]}" = verification ]; }; then
                continue
            fi
            if ! printf '%s ' "$deps" | grep -Fq "$named"; then
                warn "$id companion references $named, which $id neither owns nor depends on; update the companion or add the dependency edge"
            fi
        done
    done

    # (d) Graph leaves in a goal that owns a verification unit: a
    #     non-verification unit nothing depends on is unverified work.
    for goal_name in "${!goal_units[@]}"; do
        goal_has_verifier=false
        for id in ${goal_units[$goal_name]}; do
            [ "${unit_type[$id]}" = verification ] && goal_has_verifier=true
        done
        [ "$goal_has_verifier" = true ] || continue
        for id in ${goal_units[$goal_name]}; do
            [ "${unit_type[$id]}" = verification ] && continue
            dependent=false
            for candidate in "${unit_ids[@]}"; do
                printf '%s' "${unit_depends[$candidate]}" | grep -Fq "$id" && { dependent=true; break; }
            done
            if [ "$dependent" = false ]; then
                warn "$id is a graph leaf in a goal that owns a verification unit; nothing depends on it, so nothing verifies its output"
            fi
        done
    done

    # (e) Roster vs inventory: a goal's §9.x Owned-work-units roster must be
    #     exactly the set the inventory assigns to it. The authoritative roster
    #     is the summary paragraph §9.1 (ids written bare, before the em-dash),
    #     NOT the per-unit blurbs §9.2+, which accumulate as units are added and
    #     are not guaranteed to exist for every unit (report 13). Extract the
    #     union of the leading comma-separated run of §9.1 and the backticked
    #     blurb ids, then compare both directions.
    for goal_name in "${!goal_units[@]}"; do
        goal_file="$plan_dir/$goal_name/goal.md"
        [ -f "$goal_file" ] || continue
        # Assigned set: the inventory's units for this goal.
        assigned="$(for id in ${goal_units[$goal_name]:-}; do printf '%s ' "$id"; done)"
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
        # Add backticked blurb ids (the per-unit paragraphs) across the whole
        # owned-work-units region.
        blurb_ids="$(awk '
            /^## Owned work units$/ { in_section = 1; next }
            /^## Goal-size exception$/ { in_section = 0 }
            in_section && /`W[0-9][0-9]+`/ {
                line = $0; gsub(/`/, "", line)
                n = split(line, toks, /[ `,]+/)
                for (i = 1; i <= n; i++) if (toks[i] ~ /^W[0-9][0-9]+$/) print toks[i]
            }
        ' "$goal_file")"
        roster_ids="$(printf '%s\n%s\n' "$roster_ids" "$blurb_ids" | grep -E '^W[0-9][0-9]+$' | sort -u | tr '\n' ' ')"
        # Roster-only units: a roster id that the inventory does not assign to
        # this goal. Skip ids not in the plan at all (cross-plan references such
        # as "the critical-path plan's W83" appear after the em-dash and so are
        # already excluded from the leading run, but a defensive skip is cheap).
        for rid in $roster_ids; do
            if [ -z "${unit_type[$rid]+x}" ]; then
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
    done
fi

if [ "$errors" -gt 0 ]; then
    printf 'Plan validation failed with %d error(s).\n' "$errors" >&2
    exit 1
fi

printf 'Plan validation passed: %d work units across %d goals.\n' \
    "${#unit_ids[@]}" "${#goal_units[@]}"
