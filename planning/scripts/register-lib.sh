#!/usr/bin/env bash
# MODE: DEV
# register-lib.sh — shared validation, sorting and stamping for the root
# defect register (BUGS.json) and work queue (TODO.json).
#
# Sourced, never executed. Every writer goes through here so the refusal
# rules and the rebuild exist in exactly one shape (T41).

# reg_findings <kind> <file>: print every structural finding as one line.
# kind is "bug" or "todo". Empty output means the register is sound.

# jq is the ceiling of the required runtime; every public helper refuses with
# 69 rather than half-writing a register when it is missing.
reg_require_jq() {
    command -v jq >/dev/null 2>&1 || {
        printf 'register: jq is required (it reads and writes the JSON registers); install jq and re-run\n' >&2
        exit 69
    }
}

reg_findings() {
    reg_require_jq
    local kind="$1" file="$2"
    jq -r --arg kind "$kind" '
        def st_enum:
            if $kind == "bug"
            then ["reported","confirmed","fixed","not-a-defect","wont-fix","obsolete"]
            else ["open","done","blocked","partly","decided","obsolete"] end;
        ((if $kind == "bug" then .bugs else .tasks end) // []) as $items
        | ($items | map(.id)) as $ids
        | [
            (if (($ids | length)) != (($ids | unique | length))
             then "duplicate ids" else empty end),
            ($items[] as $e
              | select(($e.parent // "") != "")
              | select(($ids | index($e.parent)) == null)
              | "\($e.id): parent \($e.parent) does not exist"),
            (range(0; ($items | length)) as $i
              | $items[$i] as $e
              | (if ((.skill // "") == "" and $kind == "bug" and $i == 0)
                 then "register does not name its schema skill" else empty end),
                (if (($e.status // "") == "") or ((st_enum | index($e.status)) == null)
                 then "\($e.id): unknown status \($e.status // "missing")" else empty end),
                (if $kind == "bug" and (($e.severity // "") == ""
                    or ((["blocking","major","minor","cosmetic"] | index($e.severity)) == null))
                 then "\($e.id): unknown severity" else empty end),
                (if (($e.priority // "") != "") and
                    ((["urgent","high","normal","low","someday"] | index($e.priority)) == null)
                 then "\($e.id): unknown priority \($e.priority)" else empty end),
                (if (($e.created_at // "") == "")
                 then "\($e.id): missing created_at" else empty end),
                (if (($e.updated_at // "") == "")
                 then "\($e.id): missing updated_at" else empty end),
                (if $kind == "bug" and (($e.reproduce // "") == "")
                 then "\($e.id): no reproduction" else empty end),
                (if $kind == "bug" and $e.status == "confirmed" and (($e.mechanism // "") == "")
                 then "\($e.id): confirmed without a mechanism" else empty end),
                (if $kind == "bug" and $e.status == "fixed" and (($e.verification // "") == "")
                 then "\($e.id): fixed without verification" else empty end)
            )
          ]
        | .[]
    ' "$file"
}

# reg_sort <kind> <file>: reorder entries worst-first in place (temp+rename).
# Bugs go priority, then severity, then numeric id. Tasks keep the queue's own
# convention: status rank, then priority, then numeric id.
reg_sort() {
    local kind="$1" file="$2" tmp
    tmp="$(mktemp "${TMPDIR:-/tmp}/register-sort.XXXXXX")"
    if [ "$kind" = bug ]; then
        jq 'def idnum: [(. | scan("[0-9]+") | tonumber)?, .];
            def prank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
            def srank: {blocking:0, major:1, minor:2, cosmetic:3}[.severity // ""] // 4;
            .bugs |= sort_by(prank, srank, (.id | idnum))' "$file" > "$tmp"
    else
        jq 'def idnum: [(. | scan("[0-9]+") | tonumber)?, .];
            def prank: {urgent:0, high:1, normal:2, low:3, someday:4}[.priority // ""] // 5;
            def srank: {open:0, blocked:1, partly:2, decided:3, done:4, obsolete:5}[.status // "open"] // 6;
            .tasks |= sort_by(srank, prank, (.id | idnum))' "$file" > "$tmp"
    fi
    mv "$tmp" "$file"
}

# reg_next_id <kind> <file>: the next free B/T number as a bare integer.
reg_next_id() {
    local kind="$1" file="$2" prefix="B"
    [ "$kind" = todo ] && prefix="T"
    jq -r --arg p "$prefix" '
        [(if $p == "B" then .bugs else .tasks end)[].id | scan("^[A-Z]*(\\d+)$") | .[0] | tonumber] | max // 0 | . + 1
    ' "$file"
}

# reg_write <kind> <file>: stamp header fields a register owes, then sort.
# Refuses (exit 65) when reg_findings still has anything to say, naming the
# rebuild that repairs structural damage a stamp cannot fix.
reg_write() {
    local kind="$1" file="$2" findings
    findings="$(reg_findings "$kind" "$file")"
    if [ -n "$findings" ]; then
        printf '%s\n' "$findings" >&2
        printf '%s: %s is not sound; run register-rebuild.sh %s "%s" first\n' \
            "${0##*/}" "$file" "$kind" "$file" >&2
        exit 65
    fi
    reg_sort "$kind" "$file"
}
