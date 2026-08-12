#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$script_dir/plan-context-lib.sh"

usage() {
    cat >&2 <<'EOF'
Usage:
  plan-context.sh init --plan-dir DIR
  plan-context.sh read --plan-dir DIR (--document ID | --unit WNN) [--view VIEW] [--format text|json] [--max-bytes N] [--max-records N] [--read-only]
  plan-context.sh check --plan-dir DIR (--entry ID | --changed | --all) [--format text|json]
  plan-context.sh refresh --plan-dir DIR (--entry ID | --stale) [--format text|json]
EOF
    return 2
}

plan_dir= command= document_id= entry_id= check_mode= refresh_mode=
document_selector_count=0 check_selector_count=0 refresh_selector_count=0
view=summary format=text read_only=0 max_bytes=32768 max_records=128

while [ "$#" -gt 0 ]; do
    case "$1" in
        init|read|check|refresh) [ -z "$command" ] || usage; command="$1"; shift ;;
        --plan-dir) [ "$#" -ge 2 ] || usage; plan_dir="$2"; shift 2 ;;
        --document) [ "$#" -ge 2 ] || usage; document_id="$2"; document_selector_count=$((document_selector_count + 1)); shift 2 ;;
        --unit) [ "$#" -ge 2 ] || usage; document_id="unit:$2"; document_selector_count=$((document_selector_count + 1)); shift 2 ;;
        --entry) [ "$#" -ge 2 ] || usage; entry_id="$2"; check_mode=entry; refresh_mode=entry; check_selector_count=$((check_selector_count + 1)); refresh_selector_count=$((refresh_selector_count + 1)); shift 2 ;;
        --changed) check_mode=changed; check_selector_count=$((check_selector_count + 1)); shift ;;
        --all) check_mode=all; check_selector_count=$((check_selector_count + 1)); shift ;;
        --stale) refresh_mode=stale; refresh_selector_count=$((refresh_selector_count + 1)); shift ;;
        --view) [ "$#" -ge 2 ] || usage; view="$2"; shift 2 ;;
        --format) [ "$#" -ge 2 ] || usage; format="$2"; shift 2 ;;
        --max-bytes) [ "$#" -ge 2 ] || usage; max_bytes="$2"; shift 2 ;;
        --max-records) [ "$#" -ge 2 ] || usage; max_records="$2"; shift 2 ;;
        --read-only) read_only=1; shift ;;
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
        trap "rm -rf \"$staging\"" EXIT
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

context_read_command() {
    local file content bounded count=0 line truncated=0
    [ "$document_selector_count" -eq 1 ] || { printf 'usage: read requires exactly one --document or --unit\n' >&2; exit 2; }
    context_entry_id "$document_id" >/dev/null
    file="$(context_resolve_document "$plan_dir" "$document_id")"
    [ -f "$file" ] || { printf 'not-found: %s\n' "$document_id" >&2; exit 66; }
    content="$(context_view_text "$file" "$view")"
    if [ "$read_only" -eq 0 ]; then
        context_with_lock "$plan_dir" context_register_processed_entry "$plan_dir" "$document_id"
    fi
    while IFS= read -r line; do
        count=$((count + 1));
        if [ "$count" -le "$max_records" ]; then printf '%s\n' "$line"; else truncated=1; break; fi
    done <<< "$content" | head -c "$max_bytes" > "${TMPDIR:-/tmp}/plan-context-read.$$"
    bounded="$(cat "${TMPDIR:-/tmp}/plan-context-read.$$")"
    rm -f "${TMPDIR:-/tmp}/plan-context-read.$$"
    [ "${#bounded}" -lt "${#content}" ] && truncated=1
    if [ "$format" = json ]; then
        bounded="${bounded//$'\\'/\\\\}"
        bounded="${bounded//$'"'/\\\"}"
        bounded="${bounded//$'\n'/\\n}"
        printf '{"command":"read","status":"ok","entry_id":"%s","view":"%s","content":"%s","next_token":%s}\n' "$document_id" "$view" "$bounded" "$([ "$truncated" -eq 1 ] && printf '"continue:%s"' "$(context_hash_file "$file")" || printf 'null')"
    else
        printf '%s\n' "$bounded"
    fi
}

context_check_command() {
    local generation changed status= fresh_count=0 changed_ids='-' affected_ids='-'
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

case "$command" in
    init) [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_init_command ;;
    read) [ "$document_selector_count" -eq 1 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_read_command ;;
    check) [ "$document_selector_count" -eq 0 ] && { [ "$refresh_selector_count" -eq 0 ] || [ "$check_mode" = entry ]; } || usage; context_check_command ;;
    refresh) [ "$document_selector_count" -eq 0 ] && { [ "$check_selector_count" -eq 0 ] || [ "$refresh_mode" = entry ]; } || usage; context_refresh_command ;;
    *) usage ;;
esac
