#!/usr/bin/env bash
# MODE: PROD
# plan-context-commands-lib.sh — the read/check subcommand bodies
# plan-context.sh dispatches to (CODE-STYLE §3, 400-line script cap;
# plan-context-lib.sh is already at its own 500-line library cap, so this
# is a second sibling rather than an addition to it).
#
# Sourced by plan-context.sh only, after plan-context-lib.sh. Reads
# script-scope variables set by plan-context.sh's own argument parsing
# (plan_dir, entry_id, document_selector_count, and friends).

set -euo pipefail
export LC_ALL=C

# JSON forbids every character in U+0000-U+001F inside a string, so a tab or a
# CR in a document made the whole payload unparseable -- `rjq` reports "control
# characters ... must be escaped" and reads nothing. Reachable through the
# sanctioned writer: update-plan-content.sh -dp keeps a tab in the paragraph
# text verbatim. Documents with no control characters take the fast path and are
# copied through untouched.
#
# The marker below stays the last comment before the function: the catalogue
# folds every comment line following a marker into that rule's reason, so a note
# placed under it is published in PORTABILITY.md as part of the rule.
# PORTABILITY(pattern-substitution-quote): bash 3.2 cannot parse
# ${var//$'"'/...} and leaks quotes out of a quoted replacement, so the JSON
# string escape runs through sed and awk instead of parameter expansion.
context_json_escape_file() {
    sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' "$1" |
        awk -v sep='\\n' '
            BEGIN { for (code = 1; code < 32; code++) ordinal[sprintf("%c", code)] = code }
            function escape_controls(line,    out, i, char) {
                out = ""
                for (i = 1; i <= length(line); i++) {
                    char = substr(line, i, 1)
                    if (char == "\t") out = out "\\t"
                    else if (char == "\r") out = out "\\r"
                    else if (char in ordinal) out = out sprintf("\\u%04x", ordinal[char])
                    else out = out char
                }
                return out
            }
            NR > 1 { printf "%s", sep }
            { printf "%s", ($0 ~ /[[:cntrl:]]/) ? escape_controls($0) : $0 }'
}

context_read_cleanup() {
    [ -z "$read_full_file" ] || rm -f "$read_full_file"
    [ -z "$read_bounded_file" ] || rm -f "$read_bounded_file" "$read_bounded_file.trimmed"
}

# One page holds whole records only, so a resume cursor always lands on a record
# boundary and consecutive pages neither overlap nor skip. The one exception is a
# single record wider than the whole byte budget: it is emitted clipped, because
# a page that can fit nothing could never advance and paging would not terminate.
context_page_records() {
    local start="$1" maxr="$2" maxb="$3" out="$4" input="$5"
    awk -v start="$start" -v maxr="$maxr" -v maxb="$maxb" -v out="$out" '
        NR <= start { next }
        {
            size = length($0) + 1
            if (emitted >= maxr) { more = 1; exit }
            if (used + size > maxb) {
                if (emitted > 0) { more = 1; exit }
                printf "%s", substr($0, 1, maxb) > (out)
                emitted = 1
                more = 1
                exit
            }
            print > (out)
            used += size
            emitted++
        }
        END { printf "%s\t%s\n", emitted + 0, more + 0 }
    ' "$input"
}

# context_read_resolve — resolve $document_id to $file/$file_hash/$view,
# validate $token against them, and compute $content and $start. Assigns into
# the caller's locals (bash dynamic scoping, the plan_map_value idiom already
# used throughout this codebase) rather than its own, since context_read_command
# declares them local and every one of them feeds its later paging step.
context_read_resolve() {
    local token_body token_rest view_status
    [ "$document_selector_count" -eq 1 ] || { printf 'usage: read requires exactly one --document or --unit\n' >&2; exit 2; }
    context_entry_id "$document_id" >/dev/null
    [ -n "$view" ] || view="$(context_default_view "$document_id")"
    # With a ROLE_ID the gate applies only if that role's allow-list says so,
    # and caps its plan-read budget. No ROLE_ID must keep the identity-free
    # probe/reader path unchanged.
    context_role_gate max_bytes
    file="$(context_resolve_document "$plan_dir" "$document_id")"
    [ -f "$file" ] || { printf 'not-found: %s\n' "$document_id" >&2; exit 66; }
    # Hash every input the entry serves, not just the primary file, so a token
    # cannot survive an edit to the inventory row a work unit is read with.
    file_hash="$(context_hash_entry "$plan_dir" "$document_id")"
    row_text=""
    case "$document_id" in
        unit:*) row_text="$(context_unit_row_text "$plan_dir" "${document_id#unit:}" || true)" ;;
    esac
    if [ -n "$token" ]; then
        # Fail closed: a cursor is only meaningful against the exact bytes and
        # view it was minted from, so resuming into shifted records is refused.
        token_body="${token#continue:}"
        token_rest="${token_body#*:}"
        [ "${token_body%%:*}" = "$file_hash" ] && [ "${token_rest%:*}" = "$view" ] || {
            printf 'stale: --token was minted against different content or view\n' >&2
            exit 65
        }
        start="${token_rest##*:}"
    fi
    # 64 is context_die: the view deliberately refuses because it cannot apply
    # here, and that must be a clean exit rather than a set -e abort with no
    # structured output. Any other non-zero is a view that ran and matched
    # nothing (changed-documents greps), which stays empty-and-ok.
    view_status=0
    content="$(context_view_text "$file" "$view" "$row_text")" || view_status=$?
    [ "$view_status" -ne 64 ] || exit 64
}

context_read_command() {
    local file file_hash content bounded start=0 emitted more page total_records truncated
    context_read_resolve
    if [ "$read_only" -eq 0 ]; then
        context_with_lock "$plan_dir" context_register_processed_entry "$plan_dir" "$document_id"
    fi
    # Budget accounting is in BYTES, and LC_ALL=C makes awk's length() a byte
    # count. mktemp plus a trap, so an interrupt cannot leak the spool or
    # collide with a reused PID.
    trap context_read_cleanup EXIT
    read_full_file="$(mktemp "${TMPDIR:-/tmp}/plan-context-read.XXXXXX")"
    read_bounded_file="$(mktemp "${TMPDIR:-/tmp}/plan-context-read.XXXXXX")"
    printf '%s\n' "$content" > "$read_full_file"
    page="$(context_page_records "$start" "$max_records" "$max_bytes" "$read_bounded_file" "$read_full_file")"
    emitted="${page%%$'\t'*}"
    more="${page##*$'\t'}"
    total_records="$(wc -l < "$read_full_file" | tr -d ' ')"
    truncated=false
    [ "$more" -eq 0 ] || truncated=true
    context_trim_partial_utf8 "$read_bounded_file"
    # Counted from the spool, not from "$bounded": in json format that variable
    # is already the escaped one-line string, so wc -l reports 1 however many
    # lines it holds.
    local shown_lines document_lines
    shown_lines="$(wc -l < "$read_bounded_file" | tr -d ' ')"
    document_lines="$(wc -l < "$file" | tr -d ' ')"
    if [ "$format" = json ]; then
        bounded="$(context_json_escape_file "$read_bounded_file")"
    else
        bounded="$(cat "$read_bounded_file")"
    fi
    context_read_cleanup
    if [ "$format" = json ]; then
        context_read_emit_json "$document_id" "$view" "$emitted" "$total_records" "$truncated" \
            "$bounded" "$file_hash" "$start" "$more" "$shown_lines" "$document_lines"
    else
        context_read_emit_text "$document_id" "$view" "$emitted" "$total_records" "$truncated" \
            "$bounded" "$file_hash" "$start" "$more" "$shown_lines" "$document_lines"
    fi
}

# context_read_emit_json <document_id> <view> <emitted> <total_records>
# <truncated> <bounded> <file_hash> <start> <more> <shown_lines>
# <document_lines> — the read command's JSON-format output.
#
# `excerpt` carries what the text format says in its excerpt= line: the
# summary view is a fixed head slice applied before paging, so it can
# withhold most of a document while next_token is legitimately null. A
# consumer reading only next_token would treat that as a complete
# document. Not a resume token, because a fixed slice cannot be resumed;
# the remedy is --view full.
context_read_emit_json() {
    local document_id="$1" view="$2" emitted="$3" total_records="$4" truncated="$5" \
        bounded="$6" file_hash="$7" start="$8" more="$9" shown_lines="${10}" document_lines="${11}"
    local excerpt_json=null
    if [ "$view" = summary ] && [ "$more" -eq 0 ] && [ "$shown_lines" -lt "$document_lines" ]; then
        excerpt_json="$(printf '{"shown_lines":%s,"document_lines":%s,"complete":false,"read_all_with":"--view full"}' \
            "$shown_lines" "$document_lines")"
    fi
    printf '{"command":"read","status":"ok","entry_id":"%s","view":"%s","returned_records":%s,"total_records":%s,"truncated":%s,"content":"%s","next_token":%s,"excerpt":%s}\n' \
        "$document_id" "$view" "$emitted" "$total_records" "$truncated" "$bounded" \
        "$([ "$more" -eq 1 ] && printf '"continue:%s:%s:%s"' "$file_hash" "$view" "$((start + emitted))" || printf 'null')" \
        "$excerpt_json"
}

# context_read_emit_text <document_id> <view> <emitted> <total_records>
# <truncated> <bounded> <file_hash> <start> <more> <shown_lines>
# <document_lines> — the read command's text-format output.
#
# The summary view is a fixed excerpt of the head of the file, so it
# truncates BEFORE paging and the page reports no withheld records. A
# reader following the documented rule -- no next_token means the
# document is fully read -- therefore concludes it has read a plan when
# it has seen the first few lines. Reviewers are steered to this view by
# default, so the excerpt has to say what it is.
context_read_emit_text() {
    local document_id="$1" view="$2" emitted="$3" total_records="$4" truncated="$5" \
        bounded="$6" file_hash="$7" start="$8" more="$9" shown_lines="${10}" document_lines="${11}"
    printf 'entry_id=%s\nview=%s\nreturned_records=%s\ntotal_records=%s\ntruncated=%s\n' \
        "$document_id" "$view" "$emitted" "$total_records" "$truncated" >&2
    printf '%s\n' "$bounded"
    [ "$more" -eq 0 ] || printf 'next_token=continue:%s:%s:%s\n' "$file_hash" "$view" "$((start + emitted))"
    if [ "$view" = summary ] && [ "$more" -eq 0 ] && [ "$shown_lines" -lt "$document_lines" ]; then
        printf 'excerpt=summary shows %s of %s line(s); this is not the whole document. Re-read with --view full (which pages, and reports next_token until nothing is withheld) before drawing a conclusion from it.\n' \
            "$shown_lines" "$document_lines"
    fi
}

context_check_command() {
    local generation changed
    local status=""
    local fresh_count=0
    local changed_ids='-'
    local affected_ids='-'
    [ -n "$check_mode" ] || { printf 'usage: check requires --entry, --changed, or --all\n' >&2; exit 2; }
    [ "$check_selector_count" -eq 1 ] || { printf 'usage: check requires exactly one --entry, --changed, or --all\n' >&2; exit 2; }
    generation="$(context_load_manifest "$plan_dir")"
    if [ "$check_mode" = all ]; then
        changed="$(context_audit_all "$plan_dir" "$generation")"
    elif [ "$check_mode" = entry ]; then
        changed="$(context_changed_entries "$plan_dir" | awk -F'\t' -v wanted="$entry_id" '$1 == wanted {print; found=1} END{if(!found) exit 1}' 2>/dev/null || true)"
    else
        changed="$(context_changed_entries "$plan_dir")"
    fi
    if [ -n "$changed" ]; then
        status=suspect
        changed_ids="$(printf '%s\n' "$changed" | cut -f1 | paste -sd, -)"
        affected_ids="$changed_ids"
    else
        status=fresh
    fi
    context_check_emit "$generation" "$status" "$changed" "$changed_ids" "$affected_ids"
}

# context_check_emit <generation> <status> <changed> <changed_ids>
# <affected_ids> — the check command's json/text output, split out of
# context_check_command to stay under CODE-STYLE §3's function cap.
context_check_emit() {
    local generation="$1" status="$2" changed="$3" changed_ids="$4" affected_ids="$5"
    if [ "$format" = json ]; then
        local changed_json='[]' id first=1 entry_json=null
        [ -n "$entry_id" ] && entry_json="\"$entry_id\""
        if [ -n "$changed" ]; then
            changed_json='['
            while IFS=$'\t' read -r id _; do
                [ -n "$id" ] || continue
                [ "$first" -eq 1 ] || changed_json+=','
                changed_json+="\"$id\""
                first=0
            done <<< "$changed"
            changed_json+=']'
        fi
        printf '{"command":"check","status":"%s","snapshot_generation":"%s","entry_id":%s,"changed_ids":%s,"affected_ids":%s,"next_token":null,"error_code":%s}\n' \
            "$status" "$generation" "$entry_json" "$changed_json" "$changed_json" "${status:+$([ "$status" = suspect ] && printf external-edit || true)}"
    else
        printf 'command=check\nstatus=%s\nsnapshot_generation=%s\nentry_id=%s\nchanged_ids=%s\naffected_ids=%s\nnext_token=-\nerror_code=%s\n' "$status" "$generation" "${entry_id:--}" "$changed_ids" "$affected_ids" "${status:+$([ "$status" = suspect ] && printf external-edit || true)}"
    fi
}
