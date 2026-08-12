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
  plan-context.sh checkpoint --plan-dir DIR --phase PHASE --state STATE --findings-file FILE --changed-files FILE --source-hash HASH --plan-hash HASH

Valid --document IDs:
  plan                 plan-description.md
  inventory            work-unit-inventory.md
  progress             progress.md
  adversarial-review   adversarial-review.md
  goal:<goal id>       <goal>/goal.md             (e.g. goal:01-build)
  step:<goal>/<step>   <goal>/steps/<step>.md     (e.g. step:01-build/02-step-verify)
  --unit WNN           the step a work unit maps to in work-unit-inventory.md
EOF
    return 2
}

plan_dir= command= document_id= entry_id= check_mode= refresh_mode= phase= checkpoint_state= findings_file= changed_files_file= source_hash= plan_hash=
document_selector_count=0 check_selector_count=0 refresh_selector_count=0
view=summary format=text read_only=0 max_bytes=32768 max_records=128

while [ "$#" -gt 0 ]; do
    case "$1" in
        init|read|check|refresh|checkpoint) [ -z "$command" ] || usage; command="$1"; shift ;;
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
    init) [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_init_command ;;
    read) [ "$document_selector_count" -eq 1 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_read_command ;;
    check) [ "$document_selector_count" -eq 0 ] && { [ "$refresh_selector_count" -eq 0 ] || [ "$check_mode" = entry ]; } || usage; context_check_command ;;
    refresh) [ "$document_selector_count" -eq 0 ] && { [ "$check_selector_count" -eq 0 ] || [ "$refresh_mode" = entry ]; } || usage; context_refresh_command ;;
    checkpoint) [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage; context_checkpoint_command ;;
    *) usage ;;
esac
