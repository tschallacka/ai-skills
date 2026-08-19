#!/usr/bin/env bash
# create-ui-validation.sh — turn a plan into a UI-validating plan: add the
# "## UI validation" section to plan-description.md, flip "UI affected" to yes,
# and create the empty ui-user-stories.md and bugs.md tables.
#
# It refuses to run twice (73): the story and bug tables are authored afterwards
# by add-ui-story.sh and the bug flow, so recreating them would drop that work.
#
# Usage:
#   create-ui-validation.sh [--plan-dir] <plan-directory> <browser-target-or-discovery-method>
#   create-ui-validation.sh --help

set -euo pipefail
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/plan-document-lib.sh
source "$script_dir/plan-document-lib.sh"
# Accept --plan-dir as a synonym for the positional plan directory: the
# bounded reader takes the flag, so a reader who learned it there is not
# refused here.
eval "set -- $(plan_hoist_plan_dir 1 "$@")"

export LC_ALL=C

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} [--plan-dir] <plan-directory> <browser-target-or-discovery-method>
       ${0##*/} --help
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -eq 2 ] || usage

plan_dir="$1"; browser_target="$2"

plan_require_directory "$plan_dir"
plan_require_safe_value 'Browser target' "$browser_target"
description="$plan_dir/plan-description.md"
stories="$plan_dir/ui-user-stories.md"
bugs="$plan_dir/bugs.md"
if [ ! -f "$description" ]; then
    printf '%s: %s\n' "${0##*/}" "Plan description not found: $description" >&2
    exit 66
fi
if [ -e "$stories" ] || [ -e "$bugs" ]; then
    printf '%s: %s\n' "${0##*/}" 'UI validation artifacts already exist' >&2
    exit 73
fi
if grep -Fqx '## UI validation' "$description"; then
    printf '%s: %s\n' "${0##*/}" 'Plan description already has a UI validation section' >&2
    exit 73
fi

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

printf 'Created %s\n' "$stories"
