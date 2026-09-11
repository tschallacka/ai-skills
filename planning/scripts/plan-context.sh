#!/usr/bin/env bash
# MODE: PROD
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
source "$script_dir/plan-context-commands-lib.sh"

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
  coverage             work-unit-inventory.md     (the coverage table)
  stories              ui-user-stories.md
  bugs                 bugs.md
  planning-bugs        planning-bugs.json
  fixes                fixes.md
  fix-keys             fix-keys.json
  approval             approval.json
  goal-progress:<goal> <goal>/progress.md         (e.g. goal-progress:01-build)
  goal:<goal id>       <goal>/goal.md             (e.g. goal:01-build)
  step:<goal>/<step>   <goal>/steps/<step>.md     (e.g. step:01-build/02-step-verify)
  --unit WNN           the step a work unit maps to in work-unit-inventory.md

Views: full, summary, metadata, ownership, instructions, acceptance, handoff,
testing, dependencies, execution-summary, changed-documents, inventory-row, validator. Default is
`full` for whole documents that are not narrative (inventory, coverage,
adversarial-review, stories, bugs, planning-bugs, fixes, fix-keys, approval) and `summary`
otherwise.

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
[[ "$max_bytes" =~ ^[1-9][0-9]*$ && "$max_records" =~ ^[1-9][0-9]*$ ]] || { printf 'usage: limits must be positive integers\n' >&2; exit 2; }
[ "$format" = text ] || [ "$format" = json ] || { printf 'usage: unsupported format\n' >&2; exit 2; }
[ -z "$token" ] || [[ "$token" =~ ^continue:[0-9a-f]{64}:[a-z][a-z-]*:[0-9]+$ ]] || { printf 'usage: malformed --token\n' >&2; exit 2; }
case "$command" in
    init) [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage ;;
    read) [ "$document_selector_count" -eq 1 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage ;;
    check) [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 1 ] && { [ "$refresh_selector_count" -eq 0 ] || [ "$check_mode" = entry ]; } || usage ;;
    refresh) [ "$document_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 1 ] && { [ "$check_selector_count" -eq 0 ] || [ "$refresh_mode" = entry ]; } || usage ;;
    checkpoint) [ "$document_selector_count" -eq 0 ] && [ "$check_selector_count" -eq 0 ] && [ "$refresh_selector_count" -eq 0 ] && [ -z "$entry_id" ] || usage ;;
    *) usage ;;
esac
[ -d "$plan_dir" ] || { printf 'not-found: plan directory %s\n' "$plan_dir" >&2; exit 66; }

context_init_command() {
    local result
    # "$BASH", not `bash`: a PATH lookup can hand this worker a different bash
    # than the one running the script, and then the body's set -e semantics are
    # not the caller's. That is how `init` on a plan with no work-unit inventory
    # came to exit 2 under one shell and 0 under another, publishing a snapshot
    # in only one of them.
    result="$(context_with_lock "$plan_dir" "$BASH" -c '
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
