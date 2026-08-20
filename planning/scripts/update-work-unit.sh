#!/usr/bin/env bash
# update-work-unit.sh — amend a work-unit inventory row and its matching step
# file in place.
#
# Usage:
#   update-work-unit.sh [--plan-dir] <plan-directory> <WNN> [<new-primary-scope>] [<new-file>]
#                       [--scope <text>] [--file <path>] [--type <type>]
#                       [--depends-on <WNN[,WNN...]|—>] [--description <text>]
#   update-work-unit.sh --help
#
# The inventory row columns are: | ID | Type | File | Primary symbol or file
# scope | Subscope | Intended change | Depends on | Goal | Step |. The third
# positional updates *Primary scope* (column 5); the optional fourth updates
# *File* (column 4). An empty positional leaves its column unchanged, so
# `"" "<path>"` updates File without touching scope. --scope/--file are the
# flag forms (equivalent to the positionals, for callers who prefer flags);
# flags update the remaining columns; nothing else changes, so coverage rows,
# the goal Owned work units section, and progress trackers are untouched —
# changing a dependency must never go through remove + re-add (that would drop
# the unit from its coverage rows and require manual repair).
#
# Exit codes: 64 bad invocation or malformed value, 66 plan directory missing.

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
Usage: ${0##*/} [--plan-dir] <plan-directory> <WNN> [<new-primary-scope>] [<new-file>] [--scope <text>] [--file <path>] [--type <type>] [--depends-on <WNN[,WNN...]|—>] [--description <text>]
       ${0##*/} --help
USAGE
    exit "$rc"
}

# -h/--help is handled in band by the flag loop below, so no pre-scan of "$@".
case "${1:-}" in
    -h|--help) usage 0 ;;
esac
[ "$#" -ge 2 ] || usage
plan_dir="$1" unit="$2"; shift 2
new_scope='' new_file='' new_type='' new_depends='' new_description=''
positional=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --scope) [ "$#" -ge 2 ] || usage; new_scope="$2"; shift 2 ;;
        --file) [ "$#" -ge 2 ] || usage; new_file="$2"; shift 2 ;;
        --type) [ "$#" -ge 2 ] || usage; new_type="$2"; shift 2 ;;
        --depends-on) [ "$#" -ge 2 ] || usage; new_depends="$2"; shift 2 ;;
        --description) [ "$#" -ge 2 ] || usage; new_description="$2"; shift 2 ;;
        -h|--help) usage 0 ;;
        -*) usage ;;
        *)
            # Positional count, not -z: an empty positional means "leave
            # unchanged" and must not consume the next positional's slot.
            positional=$((positional + 1))
            if [ "$positional" -eq 1 ]; then
                new_scope="$1"
            elif [ "$positional" -eq 2 ]; then
                new_file="$1"
            else
                usage
            fi
            shift ;;
    esac
done
if [ -z "$new_scope" ] && [ -z "$new_file" ] && [ -z "$new_type" ] && [ -z "$new_depends" ] && [ -z "$new_description" ]; then
    usage
fi

plan_require_directory "$plan_dir"
plan_git_snapshot "$plan_dir"
[[ "$unit" =~ ^W[0-9][0-9]+$ ]] || plan_die 'Work-unit ID must use WNN'
for value_name in new_scope new_file new_type new_description; do
    if [ -n "${!value_name}" ]; then
        plan_require_safe_value "$value_name" "${!value_name}"
    fi
done
if [ -n "$new_depends" ]; then
    [[ "$new_depends" =~ ^(—|-)$ ]] || [[ "$new_depends" =~ ^W[0-9][0-9]+(,[[:space:]]*W[0-9][0-9]+)*$ ]] || plan_die "Depends-on must be a comma-separated WNN list, or —"
fi
inventory="$plan_dir/work-unit-inventory.md"
[ -f "$inventory" ] || plan_die "Work-unit inventory not found: $inventory"

step_file=""
if plan_inventory_row "$inventory" "$unit"; then
    step_file="$plan_dir/$plan_inventory_goal/steps/$plan_inventory_step.md"
fi
[ -n "$step_file" ] && [ -f "$step_file" ] || plan_die "Work unit not found: $unit"

# The values about to be overwritten. Reporting only which fields changed named
# the loss without describing it: a reader could not tell that `source` became
# `docs`, and nothing anywhere held the value it replaced. See CODE-CONTRACTS.md
# contract 9a.
previous_type="$plan_inventory_type"
previous_file="$plan_inventory_file"
previous_scope="$plan_inventory_scope"
previous_depends="$plan_inventory_depends"
previous_change="$plan_inventory_change"

# One trap covers every temp: installed before the first write and never
# released with `trap - EXIT`, which would discard the library's handler (§8).
inventory_tmp="${inventory}.tmp.$$"; step_tmp="${step_file}.tmp.$$"
trap 'rm -f "$inventory_tmp" "$step_tmp"' EXIT
awk -F'|' -v wanted="$unit" -v replacement="$new_scope" -v newfile="$new_file" -v newtype="$new_type" -v newdeps="$new_depends" 'BEGIN{OFS="|"} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id); if(id==wanted){if(replacement != ""){$5=" " replacement " "}; if(newfile != ""){$4=" " newfile " "}; if(newtype != ""){$3=" " newtype " "}; if(newdeps != ""){$8=" " newdeps " "}}} {print}' "$inventory" > "$inventory_tmp"
awk -v replacement="$new_scope" -v newfile="$new_file" -v newtype="$new_type" '/^- Primary symbol or file scope:/ {if (replacement != "") {print "- Primary symbol or file scope: " replacement; next}} /^- File:/ {if (newfile != "") {print "- File: " newfile; next}} /^- Type:/ {if (newtype != "") {print "- Type: `" newtype "`"; next}} {print}' "$step_file" > "$step_tmp"
mv "$inventory_tmp" "$inventory"
mv "$step_tmp" "$step_file"
if [ -n "$new_description" ]; then
    plan_replace_paragraph "$step_file" '§ 4.1' "$new_description"
    awk -F'|' -v wanted="$unit" -v desc="$new_description" 'BEGIN{OFS="|"} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {id=$2; gsub(/^[[:space:]]+|[[:space:]]+$/, "", id); if(id==wanted){$7=" " desc " "}} {print}' "$inventory" > "$inventory_tmp"
    mv "$inventory_tmp" "$inventory"

    # The goal's owned-unit blurb is surface 5 of the seven SKILL.md lists under
    # "Resolving a finding", and it was the one this helper skipped: the row and
    # the step objective moved while the blurb kept instructing the old
    # behaviour. A goal that never wrote a blurb for this unit is reported
    # rather than silently skipped, because a missing blurb is itself a finding.
    goal_file="$plan_dir/$plan_inventory_goal/goal.md"
    if [ -f "$goal_file" ]; then
        goal_status=0
        goal_rendered="$(mktemp "${TMPDIR:-/tmp}/goal-blurb.XXXXXX")"
        plan_register_temp_file "$goal_rendered"
        awk -v wanted="$unit" -v desc="$new_description" '
            $0 ~ "^`" wanted "`" {
                printf "`%s` — %s\n", wanted, desc
                touched = 1
                next
            }
            { print }
            END { exit (touched ? 0 : 9) }
        # `|| goal_status=$?`, not a bare call: under set -e the awk exit that
        # signals "no blurb here" would abort the script with a raw 9, after the
        # inventory row had already been rewritten.
        ' "$goal_file" > "$goal_rendered" || goal_status=$?
        if [ "$goal_status" -eq 0 ]; then
            plan_atomic_write "$goal_file" < "$goal_rendered"
            rm -f "$goal_rendered"
        else
            rm -f "$goal_rendered"
            printf '%s: %s has no `%s` blurb under "## Owned work units"; the description was not synced there\n' \
                "${0##*/}" "${goal_file##*/}" "$unit" >&2
        fi
    fi
fi
changed=()
[ -n "$new_scope" ] && changed+=("scope")
[ -n "$new_file" ] && changed+=("file")
[ -n "$new_type" ] && changed+=("type")
[ -n "$new_depends" ] && changed+=("depends-on")
[ -n "$new_description" ] && changed+=("description")

# One line per replaced value, after the write, so a person can put back what
# this call overwrote. An unchanged field says nothing.
report_replaced() { # <field> <previous> <new>
    [ -n "$3" ] || return 0
    # A field re-set to the value it already held loses nothing, and saying so
    # would train the reader to skip these lines.
    [ "$2" != "$3" ] || return 0
    printf 'replaced %s %s: %s -> %s\n' "$unit" "$1" "${2:-(empty)}" "$3" >&2
}
report_replaced scope "$previous_scope" "$new_scope"
report_replaced file "$previous_file" "$new_file"
report_replaced type "$previous_type" "$new_type"
report_replaced depends-on "$previous_depends" "$new_depends"
report_replaced description "$previous_change" "$new_description"

# Changing file, scope or dependency edges can invalidate the verification unit
# that grades this one, even with the grader's own surfaces unchanged. The
# graders go to stderr: stdout carries exactly the one result line.
if [ -n "$new_file" ] || [ -n "$new_scope" ] || [ -n "$new_depends" ]; then
    graders=""; newline=$'\n'
    while IFS= read -r row; do
        plan_inventory_split "$row"
        [ "$plan_inventory_type" = verification ] || continue
        case ",${plan_inventory_depends// /}," in
            *",$unit,"*) graders="${graders:+$graders$newline}$plan_inventory_id" ;;
        esac
    done < <(plan_inventory_rows "$inventory")
    if [ -n "$graders" ]; then
        printf 'plan: %s changed behaviour; re-read its grader(s) %s — a grader checks the old behaviour until its own surfaces are updated\n' \
            "$unit" "$(printf '%s' "$graders" | tr '\n' ' ')" >&2
    fi
fi

printf 'Updated %s: %s\n' "$unit" "$(IFS=,; printf '%s' "${changed[*]}")"
