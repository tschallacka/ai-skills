#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 10 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <WNN> <type> <file|N/A> <scope> <subscope|N/A> <intended-change> <depends-on|—> <goal-name> <step-name>" >&2
    exit 64
fi

plan_dir="$1"; unit_id="$2"; unit_type="$3"; unit_file="$4"; scope="$5"; subscope="$6"
intended="$7"; depends_on="$8"; goal_name="$9"; step_name="${10}"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$plan_dir"
[[ "$unit_id" =~ ^W[0-9][0-9]+$ ]] || plan_die "Work-unit ID must use W01"
case "$unit_type" in source|markup|style|test|config|docs|data|generated|discovery|verification) ;; *) plan_die "Unsupported work-unit type: $unit_type" ;; esac
[[ "$goal_name" =~ ^[0-9][0-9]-[a-z0-9-]+$ ]] || plan_die "Goal name must use 01-kebab-case"
[[ "$step_name" =~ ^[0-9][0-9]-step-[a-z0-9-]+$ ]] || plan_die "Step name must use 01-step-kebab-case"
[ -f "$plan_dir/$goal_name/goal.md" ] || plan_die "Goal does not exist: $goal_name"
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
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"
if awk -F'|' -v wanted="$unit_id" 'function t(v){gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); return v} /^\|/ && t($2)==wanted {found=1} END {exit !found}' "$inventory"; then
    plan_die "Work-unit ID already exists: $unit_id"
fi
step_file="$plan_dir/$goal_name/steps/$step_name.md"
[ ! -e "$step_file" ] || plan_die "Step already exists: $step_file"

inventory_tmp="${inventory}.tmp.$$"
step_tmp="${step_file}.tmp.$$"
trap 'rm -f "$inventory_tmp" "$step_tmp"' EXIT
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
goal_file="$plan_dir/$goal_name/goal.md"
if grep -Fqx -- '<add work units with add-work-unit.sh>' "$goal_file"; then
    plan_replace_paragraph "$goal_file" '§ 9.1' "\`$unit_id\` — $intended"
else
    owned_tmp="${goal_file}.tmp.$$"
    trap 'rm -f "$owned_tmp"' EXIT
    awk -v addition="\`$unit_id\` — $intended" '
        /^§ 9\.[0-9]+$/ { count++ }
        /^## Goal-size exception$/ && !inserted {
            if (count == 0) exit 2
            print "§ 9." (count + 1)
            print addition
            print ""
            inserted = 1
        }
        { print }
        END {
            if (!inserted) exit 2
        }
    ' "$goal_file" > "$owned_tmp" || plan_die "Goal has no numbered Owned work units section: $goal_file"
    mv "$owned_tmp" "$goal_file"
    trap - EXIT
fi
trap - EXIT
echo "Added $unit_id and $step_file"
