#!/usr/bin/env bash
# MODE: PROD
# register-read.sh — the read side of the two registers: show one entry, list a
# filtered set, report what is open or what changed, count, or name the next id.
#
# The writers went through helpers from the start; reading did not, so every
# caller wrote its own rjq at the call site and every caller could get it wrong.
# Two did in one session (T60): reading .todos instead of .tasks reported a
# register as nearly empty while an open task sat in the other array, and a
# hand-rolled next-id crashed on the existing id T1e, which reg_next_id has
# always handled. rjq stays the reading language (T41); it is written once here,
# where a test can hold it, instead of once per caller.
#
# Usage:
#   register-read.sh <bug|todo> show <ID>
#   register-read.sh <bug|todo> list [--status S] [--priority P] [--surface TEXT] [--parent ID]
#   register-read.sh <bug|todo> report [--since ISO8601]
#   register-read.sh <bug|todo> count [--status S]
#   register-read.sh <bug|todo> next-id
#   register-read.sh --help
#
# The register file comes from BUGS_JSON / TODO_JSON, or --file PATH.
#
# Exit codes: 64 bad invocation, 66 register missing, 69 rjq missing, 1 no match
# for show.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=planning/scripts/register-lib.sh
source "$script_dir/register-lib.sh"

usage() {
    local rc="${1:-64}"
    cat <<USAGE
Usage: ${0##*/} <bug|todo> show <ID>
       ${0##*/} <bug|todo> list [--status S] [--priority P] [--surface TEXT] [--parent ID]
       ${0##*/} <bug|todo> report [--since ISO8601]
       ${0##*/} <bug|todo> count [--status S]
       ${0##*/} <bug|todo> next-id

  --file PATH   read this register instead of BUGS_JSON / TODO_JSON
USAGE
    exit "$rc"
}

case "${1:-}" in -h|--help) usage 0 ;; esac
[ "$#" -ge 2 ] || usage
kind="$1"; command="$2"; shift 2
case "$kind" in bug|todo) ;; *) usage ;; esac

# The array a register keeps its entries in. TODO.json carried a second, .todos,
# until it was folded away; naming the array in one place is what stops a reader
# picking the wrong one (T61).
items_key=tasks
default_file="${TODO_JSON:-}"
[ "$kind" = bug ] && { items_key=bugs; default_file="${BUGS_JSON:-}"; }

status='' priority='' surface='' parent='' since='' id=''
while [ "$#" -gt 0 ]; do
    case "$1" in
        --status)   [ "$#" -ge 2 ] || usage; status="$2"; shift 2 ;;
        --priority) [ "$#" -ge 2 ] || usage; priority="$2"; shift 2 ;;
        --surface)  [ "$#" -ge 2 ] || usage; surface="$2"; shift 2 ;;
        --parent)   [ "$#" -ge 2 ] || usage; parent="$2"; shift 2 ;;
        --since)    [ "$#" -ge 2 ] || usage; since="$2"; shift 2 ;;
        --file)     [ "$#" -ge 2 ] || usage; default_file="$2"; shift 2 ;;
        -h|--help)  usage 0 ;;
        -*)         usage ;;
        *)          [ -z "$id" ] || usage; id="$1"; shift ;;
    esac
done

file="$default_file"
[ -n "$file" ] || file="$(cd "$script_dir/.." && pwd)/$( [ "$kind" = bug ] && printf BUGS.json || printf TODO.json )"
[ -f "$file" ] || { printf '%s: register not found: %s\n' "${0##*/}" "$file" >&2; exit 66; }
reg_require_jq

# One filter expression for every subcommand that selects: an empty flag drops
# out of the chain rather than matching the empty string, so absent means
# unfiltered rather than "no entry has this".
select_expr='.[]
  | select($status == "" or (.status // "") == $status)
  | select($priority == "" or (.priority // "") == $priority)
  | select($parent == "" or (.parent // "") == $parent)
  | select($surface == "" or ((.surfaces // []) | join(",") | contains($surface)))'

jq_args=(--arg status "$status" --arg priority "$priority"
         --arg surface "$surface" --arg parent "$parent" --arg since "$since"
         --arg key "$items_key")

case "$command" in
    show)
        [ -n "$id" ] || usage
        __k="$items_key" rjq -e --arg id "$id" '.[env.__k][] | select(.id == $id)' "$file" || {
            printf '%s: no %s entry with id %s in %s\n' "${0##*/}" "$kind" "$id" "$file" >&2
            exit 1
        }
        ;;
    list)
        __k="$items_key" rjq -r "${jq_args[@]}" '
            .[env.__k] as $items | $items '"$select_expr"'
            | [.id, (.status // "-"), (.priority // "-"), (.severity // "-"), .title]
            | @tsv' "$file"
        ;;
    count)
        __k="$items_key" rjq -r "${jq_args[@]}" '
            [.[env.__k] '"$select_expr"'] | length' "$file"
        ;;
    next-id)
        reg_next_id "$kind" "$file"
        ;;
    report)
        # Worst-first is the register's own sort order, so a report reads the
        # file as stored rather than imposing a second opinion on urgency.
        __k="$items_key" rjq -r "${jq_args[@]}" '
            .[env.__k] as $items
            | ($items | map(select((.status // "") | test("^(open|reported|confirmed|blocked|partly)$"))) ) as $live
            | ($since == "" or ((.updated_at // .created_at // "") >= $since)) as $_
            | "\($live | length) open of \($items | length) total"
            , ""
            , ( $live[]
                | select($since == "" or ((.updated_at // .created_at // "") >= $since))
                | "\(.id)  \(.priority // "-")/\(.severity // .status // "-")  \(.title)"
                + (if (.surfaces // []) | length > 0 then "\n      surfaces: " + ((.surfaces // []) | join(", ")) else "" end) )' "$file"
        ;;
    *) usage ;;
esac
