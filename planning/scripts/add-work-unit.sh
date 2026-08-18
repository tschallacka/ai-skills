#!/usr/bin/env bash
# add-work-unit.sh — add one work unit to a plan: an inventory row, its atomic
# step file, and its `§ 9.N` entry in the owning goal's "Owned work units".
#
# All three land together or none does: every input is validated before the
# first write, and the inventory row, the step file and the goal edit are staged
# in temp files that a single EXIT trap removes.
#
# Usage:
#   add-work-unit.sh <plan-directory> --id <WNN> --type <type> --file <path|N/A> \
#       --scope <scope> --subscope <subscope|N/A> --change <intended change> \
#       --depends-on <WNN,…|—> --goal <NN-name> --step <NN-step-name>
#   add-work-unit.sh <plan-directory> <WNN> <type> <file|N/A> <scope> \
#       <subscope|N/A> <intended-change> <depends-on|—> <goal-name> <step-name>
#   add-work-unit.sh --help
#
# The second form is the deprecated positional spelling, kept working for
# existing callers. Prefer the flags: ten unnamed arguments are unreadable at
# the call site.

set -euo pipefail
export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <plan-directory> --id <WNN> --type <type> --file <path|N/A>
           --scope <scope> --subscope <subscope|N/A> --change <intended change>
           --depends-on <WNN,...|--> --goal <NN-name> --step <NN-step-name>
       ${0##*/} <plan-directory> <WNN> <type> <file|N/A> <scope> <subscope|N/A> <intended-change> <depends-on> <goal-name> <step-name>
       ${0##*/} --help

The positional form is deprecated; it is kept working for existing callers.
Types: source markup style test config docs data generated discovery verification
USAGE
    exit "$rc"
}

unit_id=""
unit_type=""
unit_file=""
scope=""
subscope=""
intended=""
depends_on=""
goal_name=""
step_name=""
flags_used=false
positional=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        --id) [ "$#" -ge 2 ] || usage; unit_id="$2"; flags_used=true; shift 2 ;;
        --type) [ "$#" -ge 2 ] || usage; unit_type="$2"; flags_used=true; shift 2 ;;
        --file) [ "$#" -ge 2 ] || usage; unit_file="$2"; flags_used=true; shift 2 ;;
        --scope) [ "$#" -ge 2 ] || usage; scope="$2"; flags_used=true; shift 2 ;;
        --subscope) [ "$#" -ge 2 ] || usage; subscope="$2"; flags_used=true; shift 2 ;;
        --change) [ "$#" -ge 2 ] || usage; intended="$2"; flags_used=true; shift 2 ;;
        --depends-on) [ "$#" -ge 2 ] || usage; depends_on="$2"; flags_used=true; shift 2 ;;
        --goal) [ "$#" -ge 2 ] || usage; goal_name="$2"; flags_used=true; shift 2 ;;
        --step) [ "$#" -ge 2 ] || usage; step_name="$2"; flags_used=true; shift 2 ;;
        --) shift; break ;;
        -*) printf '%s: unknown option: %s\n' "${0##*/}" "$1" >&2; usage ;;
        *) positional+=("$1"); shift ;;
    esac
done
while [ "$#" -gt 0 ]; do positional+=("$1"); shift; done

set -- ${positional[@]+"${positional[@]}"}
if [ "$flags_used" = true ]; then
    [ "$#" -eq 1 ] || usage
    plan_dir="$1"
    for required in unit_id unit_type unit_file scope subscope intended depends_on goal_name step_name; do
        [ -n "${!required}" ] || usage
    done
else
    # Deprecated positional form.
    [ "$#" -eq 10 ] || usage
    plan_dir="$1"; unit_id="$2"; unit_type="$3"; unit_file="$4"; scope="$5"; subscope="$6"
    intended="$7"; depends_on="$8"; goal_name="$9"; step_name="${10}"
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"
source "$script_dir/plan-reconcile-lib.sh"

plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
[[ "$unit_id" =~ ^W[0-9][0-9]+$ ]] || plan_die "Work-unit ID must use W01"
case "$unit_type" in source|markup|style|test|config|docs|data|generated|discovery|verification) ;; *) plan_die "Unsupported work-unit type: $unit_type" ;; esac
[[ "$goal_name" =~ ^[0-9][0-9]-[a-z0-9-]+$ ]] || plan_die "Goal name must use 01-kebab-case"
[[ "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]] || plan_die "Step name must use 01-step-kebab-case"
goal_file="$plan_dir/$goal_name/goal.md"
if [ ! -f "$goal_file" ]; then
    printf '%s: %s\n' "${0##*/}" "Goal does not exist: $goal_name" >&2
    exit 66
fi
for value_name in unit_file scope subscope intended depends_on; do
    plan_require_safe_value "$value_name" "${!value_name}"
done
if [ "$unit_type" = verification ]; then
    [ "$unit_file" = N/A ] || plan_die "Verification work units must use file N/A"
else
    [ "$unit_file" != N/A ] || plan_die "Only verification work units may use file N/A"
fi
[[ "$unit_file" != *'*'* && "$unit_file" != */ ]] || plan_die "File must be one concrete file, not a glob or directory"

inventory="$plan_dir/work-unit-inventory.md"
if [ ! -f "$inventory" ]; then
    printf '%s: %s\n' "${0##*/}" "Work-unit inventory not found: $inventory" >&2
    exit 66
fi
if awk -F'|' -v wanted="$unit_id" 'function t(v){gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v} /^\|/ && t($2)==wanted {found=1} END {exit !found}' "$inventory"; then
    printf '%s: %s\n' "${0##*/}" "Work-unit ID already exists: $unit_id" >&2
    exit 73
fi
step_file="$plan_dir/$goal_name/steps/$step_name.md"
if [ -e "$step_file" ]; then
    printf '%s: %s\n' "${0##*/}" "Step already exists: $step_file" >&2
    exit 73
fi

# One trap for all three staging files, installed before any of them exists and
# never released: re-trapping for the goal edit used to replace this handler and
# leak the inventory and step temps on a later failure.
inventory_tmp="${inventory}.tmp.$$"
step_tmp="${step_file}.tmp.$$"
owned_tmp="${goal_file}.tmp.$$"
trap 'rm -f "$inventory_tmp" "$step_tmp" "$owned_tmp"' EXIT
awk -v row="| $unit_id | $unit_type | \`$unit_file\` | \`$scope\` | \`$subscope\` | $intended | $depends_on | $goal_name | $step_name |" '
    /^## Decomposition review$/ && !inserted { print row; print ""; inserted = 1 }
    { print }
    END { if (!inserted) exit 2 }
' "$inventory" > "$inventory_tmp" || plan_die "Inventory has no Decomposition review section"
{
    printf '# Step: %s\n\n' "$step_name"
    printf '## Ownership\n\n- Goal: `%s`\n- Work unit: `%s`\n- Type: `%s`\n\n' "$goal_name" "$unit_id" "$unit_type"
    printf '## Change target\n\n- File: `%s`\n- Primary symbol or file scope: `%s`\n- Subscope: `%s`\n\n' "$unit_file" "$scope" "$subscope"
    printf '## Objective\n\n§ 4.1\n%s\n\n' "$intended"
    printf '## Instructions\n\n§ 5.1\n<direct action on this one target>\n\n'
    printf '## Acceptance criteria\n\n§ 6.1\n<observable result for this target>\n\n'
    printf '## Handoff\n\n§ 7.1\n<what the next named work unit can rely on>\n\n'
    printf '## Atomicity check\n\n- [x] This step owns exactly one inventory work unit.\n- [x] No other file, symbol, test target, or verification flow changes here.\n- [x] Any follow-on target has a separately named work unit and step.\n'
} > "$step_tmp"
mv "$inventory_tmp" "$inventory"
mv "$step_tmp" "$step_file"
if grep -Fqx -- '<add work units with add-work-unit.sh>' "$goal_file"; then
    plan_replace_paragraph "$goal_file" '§ 9.1' "\`$unit_id\` — $intended"
else
    # One scan yields "<next-label> <insert-after-line>"; both need the whole
    # § 9 section. Computing either against a truncated view duplicates § 9.2
    # from the third unit onward, or reverses the section's order.
    read -r next_label insert_after < <(awk '
        /^§ 9\.[0-9]+$/ {
            n = $2; sub(/.*\./, "", n)
            if (n + 0 > max) max = n + 0
            last_label = NR; found = 1
            next
        }
        last_label && NR == last_label + 1 {
            content_end = NR
            last_label = 0
            next
        }
        content_end && $0 == "" {
            insert_after = NR
            content_end = 0
        }
        END {
            if (!found) exit 1
            print max + 1, insert_after
        }
    ' "$goal_file") || plan_die "Goal has no numbered Owned work units section: $goal_file"
    [ -n "$insert_after" ] || plan_die "Goal Owned work units section has no paragraph to append after: $goal_file"
    awk -v addition="\`$unit_id\` — $intended" -v label="$next_label" -v after="$insert_after" '
        NR == after {
            print
            print "§ 9." label
            print addition
            print ""
            inserted = 1
            next
        }
        { print }
        END {
            if (!inserted) exit 2
        }
    ' "$goal_file" > "$owned_tmp" || plan_die "Goal has no numbered Owned work units section: $goal_file"
    mv "$owned_tmp" "$goal_file"
fi
printf 'Added %s and %s\n' "$unit_id" "$step_file"
testing_required="$(plan_testing_requirement_for_goal "$goal_file")"
if [ "$testing_required" = yes ] && [ "$unit_type" != test ] && [ "$unit_type" != verification ]; then
    printf 'Reminder: goal %s requires testing; continue with its test/proof step. Review any existing testing instructions for accuracy and completeness.\n' "$goal_name" >&2
fi
plan_rebuild_goal_progress "$script_dir" "$plan_dir/$goal_name" "$goal_name"
plan_rebuild_plan_progress "$script_dir" "$plan_dir"
