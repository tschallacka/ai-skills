#!/usr/bin/env bash
# plan-inventory-lib — the one reader of the work-unit inventory table.
#
# The inventory's data columns are addressed by number here and nowhere else,
# so adding a column touches this file alone. Readers get a fixed TSV whose
# cells are trimmed at both ends, stripped of surrounding backticks, and have
# tabs folded to a space so the TSV always splits into exactly nine fields.
#
# Usage: sourced (by plan-document-lib.sh; never executed).
#   plan_inventory_rows  <inventory>              # fixed TSV, one line per row
#   plan_inventory_split <tsv-line>               # fill the channel from a line
#   plan_inventory_row   <inventory> <id>         # fill the channel, 1 if absent
#
# ---- quoted: the fixed TSV field order ----
# id  type  file  scope  subscope  change  depends  goal  step
# ---- end quoted ----
#
# Iterate with plan_inventory_rows piped into plan_inventory_split: one awk for
# the whole table and no fork per row. plan_inventory_row is the single-lookup
# form and costs one awk per call.

set -euo pipefail

# Value channel for plan_inventory_row and plan_inventory_split. Not local: the
# caller reads them.
plan_inventory_id=""
plan_inventory_type=""
plan_inventory_file=""
plan_inventory_scope=""
plan_inventory_subscope=""
plan_inventory_change=""
plan_inventory_depends=""
plan_inventory_goal=""
plan_inventory_step=""
plan_inventory_tsv=""

plan_inventory_die() {
    # plan-document-lib.sh owns plan_die; stay usable when sourced on its own.
    if command -v plan_die >/dev/null 2>&1; then
        plan_die "$1" "${2:-70}"
    fi
    printf '%s: %s\n' "${0##*/}" "$1" >&2
    exit "${2:-70}"
}

# Held in a variable rather than a heredoc function so a lookup costs one fork
# instead of two. Both trim anchors are required: an unanchored
# `[[:space:]]+$` alternative strips interior whitespace runs too.
plan_inventory_awk_program='
function cell(v) {
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
    gsub(/^`|`$/, "", v)
    gsub(/\t/, " ", v)
    return v
}
/^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {
    id = cell($2)
    if (wanted != "" && id != wanted) next
    print id "\t" cell($3) "\t" cell($4) "\t" cell($5) "\t" cell($6) "\t" \
        cell($7) "\t" cell($8) "\t" cell($9) "\t" cell($10)
    if (wanted != "") exit
}
'

# The one awk invocation that knows the inventory's field numbers. An empty
# <id> emits every row.
# A missing inventory is refused here rather than left to awk. Bare awk writes
# its own message and exits 2, and whether that aborted the caller depended on
# the bash running it: `plan-context.sh init` on a plan with no inventory exited
# 2 under bash 5.3 and 0 under bash 3.2, publishing a snapshot either way.
plan_inventory_scan() {
    [ -f "$1" ] || plan_inventory_die \
        "work-unit inventory not found: $1 -- create it with create-work-unit-inventory.sh, or skip the read when the plan has no inventory yet" 66
    awk -F'|' -v wanted="$2" "$plan_inventory_awk_program" "$1"
}

# Every work-unit row of <inventory>, in file order, as the fixed TSV.
plan_inventory_rows() {
    plan_inventory_scan "$1" ''
}

# Fill the value channel from one TSV line. No fork, so this is the loop body
# form. A short line leaves the trailing fields empty.
plan_inventory_split() {
    plan_inventory_tsv="$1"
    IFS=$'\t' read -r plan_inventory_id plan_inventory_type plan_inventory_file \
        plan_inventory_scope plan_inventory_subscope plan_inventory_change \
        plan_inventory_depends plan_inventory_goal plan_inventory_step <<< "$1"
}

# One row by id into the value channel; 1 when the id has no row. The first
# match wins: a duplicated id is a malformed inventory, which validate-plan
# rejects.
plan_inventory_row() {
    local row
    row="$(plan_inventory_scan "$1" "$2")"
    if [ -z "$row" ]; then
        plan_inventory_split ''
        return 1
    fi
    plan_inventory_split "$row"
}
