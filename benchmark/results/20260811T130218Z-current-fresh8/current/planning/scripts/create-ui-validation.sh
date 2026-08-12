#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "Usage: $(basename "$0") <plan-directory> <browser-target-or-discovery-method>" >&2
    exit 64
fi

plan_dir="$1"; browser_target="$2"
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-document-lib.sh"

plan_require_directory "$plan_dir"
plan_require_safe_value 'Browser target' "$browser_target"
description="$plan_dir/plan-description.md"
stories="$plan_dir/ui-user-stories.md"
bugs="$plan_dir/bugs.md"
[ -f "$description" ] || plan_die "Plan description not found: $description"
[ ! -e "$stories" ] && [ ! -e "$bugs" ] || plan_die "UI validation artifacts already exist"
grep -Fqx '## UI validation' "$description" && plan_die "Plan description already has a UI validation section"

description_tmp="${description}.tmp.$$"
trap 'rm -f "$description_tmp"' EXIT
awk -v target="$browser_target" '
    /^## Adversarial review$/ && !inserted {
        print "## UI validation"
        print ""
        print "- Required: yes"
        print "- Browser target: " target
        print "- Story artifact: `ui-user-stories.md`"
        print ""
        inserted = 1
    }
    { print }
    END { if (!inserted) exit 2 }
' "$description" > "$description_tmp" || plan_die "Plan description has no Adversarial review section"
mv "$description_tmp" "$description"
trap - EXIT
plan_replace_field "$description" 'UI affected' yes

{
    printf '# UI user stories: %s\n\n' "$(basename "$plan_dir")"
    printf '| ID | Persona / precondition | Browser actions | Interaction evidence | Expected observable result | Status | Evidence | Related work units | Run cache |\n'
    printf '|---|---|---|---|---|---|---|---|---|\n'
} > "$stories"
{
    printf '# UI bugs: %s\n\n' "$(basename "$plan_dir")"
    printf '| ID | Story | Severity | Reproduction/evidence | Investigation goal | Fix goal | Retest story | Status |\n'
    printf '|---|---|---|---|---|---|---|---|\n'
} > "$bugs"

echo "Created UI validation artifacts for $plan_dir"
