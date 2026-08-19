#!/usr/bin/env bash
# plan-context.sh — bounded reader and freshness gate for one plan's documents.
#
# Owns the plan-context cache: `init` snapshots every plan document with its
# hash, `read` returns one PAGE of a document view under byte and record
# budgets, `check` reports which snapshotted documents drifted, `refresh`
# re-snapshots, and `checkpoint` records a phase state. Budgets are accounted in
# BYTES and bound each page, not the document.
#
# A page that withholds records returns `next_token`; feeding it back through
# `--token` resumes at the next record. The token carries the document's
# SHA-256 and the view it was minted for, so a token replayed against changed
# content is refused (65) instead of resuming into shifted records.
#
# Usage:
#   plan-context.sh init|read|check|refresh|checkpoint --plan-dir DIR [...]
#   plan-context.sh --help
#
# Exit codes: 2 bad invocation, 64 refused by the ROLE_ID reader allow-list,
# 65 stale --token (document or view no longer matches), 66 plan directory or
# document missing.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-context-lib.sh"

usage() {
    local rc="${1:-2}"
    cat <<'EOF'
Usage:
  plan-context.sh init --plan-dir DIR
  plan-context.sh read --plan-dir DIR (--document ID | --unit WNN) [--view VIEW] [--token TOKEN] [--format text|json] [--max-bytes N] [--max-records N] [--read-only]
  plan-context.sh check --plan-dir DIR (--entry ID | --changed | --all) [--format text|json]
  plan-context.sh refresh --plan-dir DIR (--entry ID | --stale) [--format text|json]
  plan-context.sh checkpoint --plan-dir DIR --phase PHASE --state STATE --findings-file FILE --changed-files FILE --source-hash HASH --plan-hash HASH

Valid --document IDs:
  plan                 plan-description.md
  inventory            work-unit-inventory.md
  progress             progress.md
  adversarial-review   adversarial-review.md
  goal:<goal id>       <goal>/goal.md             (e.g. goal:01-build)
  step:<goal>/<step>   <goal>/steps/<step>.md     (e.g. step:01-build/02-step-verify)
  --unit WNN           the step a work unit maps to in work-unit-inventory.md

Views: full, summary, metadata, ownership, instructions, acceptance, handoff,
testing, dependencies, changed-documents, validator. Default is `full` for
inventory and adversarial-review, `summary` otherwise.

Paging: a page that withholds records reports next_token; pass it back as
--token to resume. A token is refused (65) once the document or view it was
minted against no longer matches.
EOF
    exit "$rc"
}

plan_dir=""
command=""
document_id=""
entry_id=""
check_mode=""
refresh_mode=""
phase=""
checkpoint_state=""
findings_file=""
changed_files_file=""
source_hash=""
plan_hash=""
document_selector_count=0
check_selector_count=0
refresh_selector_count=0
view=""
token=""
format=text
read_only=0
max_bytes=32768
max_records=128
# Spool paths for the bounded read, at script scope so the EXIT trap can still
# see them after context_read_command returns (a `local` would be out of scope
# by then and trip set -u inside the handler).
read_full_file=""
read_bounded_file=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        -h|--help) usage 0 ;;
        init|read|check|refresh|checkpoint) [ -z "$command" ] || usage; command="$1"; shift ;;
        --plan-dir) [ "$#" -ge 2 ] || usage; plan_dir="$2"; shift 2 ;;
        --document) [ "$#" -ge 2 ] || usage; document_id="$2"; document_selector_count=$((document_selector_count + 1)); shift 2 ;;
        --unit) [ "$#" -ge 2 ] || usage; document_id="unit:$2"; document_selector_count=$((document_selector_count + 1)); shift 2 ;;
        --entry) [ "$#" -ge 2 ] || usage; entry_id="$2"; check_mode=entry; refresh_mode=entry; check_selector_count=$((check_selector_count + 1)); refresh_selector_count=$((refresh_selector_count + 1)); shift 2 ;;
        --changed) check_mode=changed; check_selector_count=$((check_selector_count + 1)); shift ;;
        --all) check_mode=all; check_selector_count=$((check_selector_count + 1)); shift ;;
        --stale) refresh_mode=stale; refresh_selector_count=$((refresh_selector_count + 1)); shift ;;
        --view) [ "$#" -ge 2 ] || usage; view="$2"; shift 2 ;;
        --token) [ "$#" -ge 2 ] || usage; token="$2"; shift 2 ;;
        --format) [ "$#" -ge 2 ] || usage; format="$2"; shift 2 ;;
        --max-bytes) [ "$#" -ge 2 ] || usage; max_bytes="$2"; shift 2 ;;
        --max-records) [ "$#" -ge 2 ] || usage; max_records="$2"; shift 2 ;;
        --read-only) read_only=1; shift ;;
        --phase) [ "$#" -ge 2 ] || usage; phase="$2"; shift 2 ;;
        --state) [ "$#" -ge 2 ] || usage; checkpoint_state="$2"; shift 2 ;;
        --findings-file) [ "$#" -ge 2 ] || usage; findings_file="$2"; shift 2 ;;
        --changed-files) [ "$#" -ge 2 ] || usage; changed_files_file="$2"; shift 2 ;;
        --source-hash) [ "$#" -ge 2 ] || usage; source_hash="$2"; shift 2 ;;
        --plan-hash) [ "$#" -ge 2 ] || usage; plan_hash="$2"; shift 2 ;;
        *) usage ;;
    esac
done

[ -n "$command" ] && [ -n "$plan_dir" ] || usage
[ -d "$plan_dir" ] || { printf 'not-found: plan directory %s\n' "$plan_dir" >&2; exit 66; }
[[ "$max_bytes" =~ ^[1-9][0-9]*$ && "$max_records" =~ ^[1-9][0-9]*$ ]] || { printf 'usage: limits must be positive integers\n' >&2; exit 2; }
[ "$format" = text ] || [ "$format" = json ] || { printf 'usage: unsupported format\n' >&2; exit 2; }

context_init_command() {
    local result
    result="$(context_with_lock "$plan_dir" bash -c '
        set -euo pipefail
        plan_dir="$1"; source "$2"
        generation="$(context_allocate_generation "$plan_dir")"
        staging="$(mktemp -d "$(context_root "$plan_dir")/init.XXXXXX")"
        trap '\''rm -rf "$staging"'\'' EXIT
        context_build_index "$plan_dir" "$staging/index.tsv"
        printf "schema_version\\tgenerator_version\\tresult_schema_version\\n%s\\t%s\\t%s\\n" "$context_schema_version" "$context_generator_version" "$context_result_schema_version" > "$staging/manifest.tsv"
        mkdir -p "$staging/entries"
        context_publish_snapshot "$plan_dir" "$generation" "$staging/index.tsv" "$staging/manifest.tsv" "$staging/entries"
        printf "command=init\nstatus=fresh\nsnapshot_generation=%s\nentry_id=-\nchanged_ids=-\naffected_ids=-\nnext_token=-\nerror_code=-\n" "$generation"
    ' _ "$plan_dir" "$script_dir/plan-context-lib.sh")"
    if [ "$format" = json ]; then
        context_write_json_result init fresh "$(printf '%s\n' "$result" | awk -F= '$1 == "snapshot_generation" {print $2}')"
    else
        printf '%s\n' "$result"
    fi
}

# --max-bytes is a byte budget, so `head -c` can split one of the multi-byte
# glyphs plan documents are full of (§ 💤 ⏳ ✅ —) and emit invalid UTF-8 inside
# the JSON "content" string. Drop any sequence shorter than it declares.
context_trim_partial_utf8() {
    local file="$1" size bytes byte index count need have keep
    size="$(wc -c < "$file" | tr -d ' ')"
    [ "$size" -gt 0 ] || return 0
    bytes="$(tail -c 4 "$file" | od -An -tu1 | tr '\n' ' ')"
    # Intentional word split: od emits space-separated decimal byte values.
    # shellcheck disable=SC2086
    set -- $bytes
    count="$#"
    index="$count"
    while [ "$index" -ge 1 ]; do
        eval "byte=\${$index}"
        [ "$byte" -ge 128 ] || return 0
        if [ "$byte" -ge 192 ]; then
            if [ "$byte" -ge 240 ]; then need=4
            elif [ "$byte" -ge 224 ]; then need=3
            else need=2
            fi
            have=$((count - index + 1))
            if [ "$have" -lt "$need" ]; then
                keep=$((size - have))
                head -c "$keep" "$file" > "$file.trimmed"
                mv -f "$file.trimmed" "$file"
            fi
            return 0
        fi
        index=$((index - 1))
    done
}

# PORTABILITY(pattern-substitution-quote): bash 3.2 cannot parse
# ${var//$'"'/...} and leaks quotes out of a quoted replacement, so the JSON
# string escape runs through sed and awk instead of parameter expansion.
context_json_escape_file() {
    sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' "$1" |
        awk -v sep='\\n' 'NR > 1 { printf "%s", sep } { printf "%s", $0 }'
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

context_read_command() {
    local file file_hash content bounded start=0 emitted more page token_body token_rest
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
        [[ "$token" =~ ^continue:[0-9a-f]{64}:[a-z][a-z-]*:[0-9]+$ ]] || { printf 'usage: malformed --token\n' >&2; exit 2; }
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
    context_trim_partial_utf8 "$read_bounded_file"
    if [ "$format" = json ]; then
        bounded="$(context_json_escape_file "$read_bounded_file")"
    else
        bounded="$(cat "$read_bounded_file")"
    fi
    context_read_cleanup
    if [ "$format" = json ]; then
        printf '{"command":"read","status":"ok","entry_id":"%s","view":"%s","content":"%s","next_token":%s}\n' \
            "$document_id" "$view" "$bounded" \
            "$([ "$more" -eq 1 ] && printf '"continue:%s:%s:%s"' "$file_hash" "$view" "$((start + emitted))" || printf 'null')"
    else
        printf '%s\n' "$bounded"
        [ "$more" -eq 0 ] || printf 'next_token=continue:%s:%s:%s\n' "$file_hash" "$view" "$((start + emitted))"
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

context_refresh_command() {
    [ "$refresh_selector_count" -eq 1 ] || { printf 'usage: refresh requires exactly one --entry or --stale\n' >&2; exit 2; }
    context_init_command
}

context_checkpoint_command() {
    [[ "$phase" =~ ^(drafting|review|correction|validation)$ ]] || { printf 'usage: invalid checkpoint phase\n' >&2; exit 2; }
    [[ "$checkpoint_state" =~ ^(in_progress|blocked|complete)$ ]] || { printf 'usage: invalid checkpoint state\n' >&2; exit 2; }
    [ -s "$findings_file" ] && [ -f "$changed_files_file" ] || { printf 'usage: checkpoint input file missing\n' >&2; exit 2; }
    [[ "$source_hash" =~ ^[0-9a-fA-F]{64}$ && "$plan_hash" =~ ^[0-9a-fA-F]{64}$ ]] || { printf 'usage: checkpoint hashes must be SHA-256\n' >&2; exit 2; }
    local root tmp now
    root="$(context_root "$plan_dir")/checkpoints"
    mkdir -p "$root"
    now="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    tmp="$root/$phase.json.tmp.$$"
    {
        printf '{"schema_version":"1.4.2","run_id":"%s","revision":"%s","phase":"%s","state":"%s","open_findings":[' "${RUN_ID:-local}" "${REVISION:-local}" "$phase" "$checkpoint_state"
        awk 'NR > 1 {gsub(/["\\]/,"_"); printf "%s\"%s\"", (n++ ? "," : ""), $0}' "$findings_file"
        printf '],"next_action":"%s","changed_files":[' "${NEXT_ACTION:-continue}"
        awk '{gsub(/["\\]/,"_"); printf "%s\"%s\"", (n++ ? "," : ""), $0}' "$changed_files_file"
        printf '],"source_hash":"%s","plan_hash":"%s","created_at":"%s","updated_at":"%s"}\n' "$source_hash" "$plan_hash" "$now" "$now"
    } > "$tmp"
    mv "$tmp" "$root/$phase.json"
    printf 'checkpoint=%s\nstate=%s\npath=%s\n' "$phase" "$checkpoint_state" "$root/$phase.json"
}

case "$command" in
    init) context_role_gate - ; [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_init_command ;;
    read) [ "$document_selector_count" -eq 1 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_read_command ;;
    check) context_role_gate - ; [ "$document_selector_count" -eq 0 ] && { [ "$refresh_selector_count" -eq 0 ] || [ "$check_mode" = entry ]; } || usage; context_check_command ;;
    refresh) context_role_gate - ; [ "$document_selector_count" -eq 0 ] && { [ "$check_selector_count" -eq 0 ] || [ "$refresh_mode" = entry ]; } || usage; context_refresh_command ;;
    checkpoint) context_role_gate - ; [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_checkpoint_command ;;
    *) usage ;;
esac
