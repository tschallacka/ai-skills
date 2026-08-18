#!/usr/bin/env bash
# plan-context-lib.sh — shared implementation for the bounded planning context
# cache: snapshot generations, the plan-directory lock, document resolution,
# views, the freshness index, and the per-role reader gate.
#
# Usage:
#   source "$script_dir/plan-context-lib.sh"    # sourced, never executed
#
# Sourced by plan-context.sh (and by the `bash -c` worker it spawns for init).
# Every function here is prefixed `context_` so nothing shadows a caller's name.

set -euo pipefail
export LC_ALL=C

context_schema_version=1
context_generator_version=1
context_result_schema_version=1

context_die() { printf '%s\n' "$*" >&2; return 64; }

context_hash_file() {
    local file="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$file" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$file" | awk '{print $1}'
    elif command -v openssl >/dev/null 2>&1; then
        openssl dgst -sha256 "$file" | awk '{print $NF}'
    else
        context_die "No SHA-256 command available (sha256sum, shasum, or openssl)"
    fi
}

context_plan_dir() { printf '%s\n' "$1"; }
context_root() { printf '%s/context\n' "$1"; }
context_snapshot_root() { printf '%s/context/snapshots\n' "$1"; }
context_current_file() { printf '%s/context/current\n' "$1"; }
context_processed_file() { printf '%s/context/processed.tsv\n' "$1"; }
context_lock_dir() { printf '%s/context/.lock\n' "$1"; }

# Fractional sleep is a GNU/BSD extension, not POSIX: busybox ash's sleep
# rejects "0.1" and, under set -e, context_with_lock would die instead of
# retrying. Probe once and fall back to a whole second.
if sleep 0.0 2>/dev/null; then
    context_lock_retry_delay=0.1
else
    context_lock_retry_delay=1
fi

context_with_lock() {
    local plan_dir="$1"; shift
    local lock; lock="$(context_lock_dir "$plan_dir")"
    local timeout="${CONTEXT_LOCK_TIMEOUT:-10}" start now
    start="$(date +%s)"
    mkdir -p "$(context_root "$plan_dir")"
    while ! mkdir "$lock" 2>/dev/null; do
        if [ -f "$lock/pid" ]; then
            local owner
            owner="$(cat "$lock/pid" 2>/dev/null || true)"
            if [ -n "$owner" ] && ! kill -0 "$owner" 2>/dev/null; then
                rm -rf "$lock"
                continue
            fi
        fi
        now="$(date +%s)"
        [ $((now - start)) -lt "$timeout" ] || context_die "lock: context lock timeout: $lock"
        sleep "$context_lock_retry_delay"
    done
    printf '%s\n' "$$" > "$lock/pid"
    trap 'rm -rf "$lock"' RETURN
    "$@"
    local status=$?
    rm -rf "$lock"
    trap - RETURN
    return "$status"
}

context_allocate_generation() {
    local plan_dir="$1"; mkdir -p "$(context_snapshot_root "$plan_dir")"
    local max=0 candidate number
    for candidate in "$(context_snapshot_root "$plan_dir")"/*; do
        [ -d "$candidate" ] || continue
        number="$(basename "$candidate")"
        [[ "$number" =~ ^[0-9]+$ ]] || continue
        [ "$number" -gt "$max" ] && max="$number"
    done
    printf '%s\n' $((max + 1))
}

context_resolve_document() {
    local plan_dir="$1" document_id="$2"
    case "$document_id" in
        plan) printf '%s/plan-description.md\n' "$plan_dir" ;;
        inventory) printf '%s/work-unit-inventory.md\n' "$plan_dir" ;;
        progress) printf '%s/progress.md\n' "$plan_dir" ;;
        adversarial-review) printf '%s/adversarial-review.md\n' "$plan_dir" ;;
        goal:*) printf '%s/%s/goal.md\n' "$plan_dir" "${document_id#goal:}" ;;
        step:*)
            local value goal step
            value="${document_id#step:}"; goal="${value%%/*}"; step="${value#*/}"
            [ "$goal" != "$value" ] && [ -n "$step" ] || context_die "usage: invalid step entry: $document_id"
            printf '%s/%s/steps/%s.md\n' "$plan_dir" "$goal" "$step"
            ;;
        unit:W*)
            local unit="${document_id#unit:}" row goal step
            row="$(awk -F'|' -v wanted="$unit" 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {if(trim($2)==wanted) print trim($9) "\t" trim($10)}' "$plan_dir/work-unit-inventory.md")"
            IFS=$'\t' read -r goal step <<< "$row"
            [ -n "${goal:-}" ] && [ -n "${step:-}" ] || context_die "not-found: work unit $unit"
            printf '%s/%s/steps/%s.md\n' "$plan_dir" "$goal" "$step"
            ;;
        *) context_die "usage: unsupported entry id: $document_id" ;;
    esac
}

context_entry_id() {
    case "$1" in
        plan|goal:*|step:*|unit:W*|inventory|progress|adversarial-review) printf '%s\n' "$1" ;;
        *) context_die "usage: unsupported entry id: $1" ;;
    esac
}

context_view_text() {
    local file="$1" view="$2"
    case "$view" in
        metadata) sed -n '/^## Ownership$/,/^## Objective$/p; /^## Change target$/,/^## Objective$/p' "$file" | sed '$d' ;;
        summary) sed -n '1,12p' "$file" ;;
        instructions) awk '/^## Instructions$/{seen=1; next} seen && /^§ 5\.1$/{getline; print; exit}' "$file" ;;
        acceptance) awk '/^## Acceptance criteria$/{seen=1; next} seen && /^§ 6\.1$/{getline; print; exit}' "$file" ;;
        handoff) awk '/^## Handoff$/{seen=1; next} seen && /^§ 7\.1$/{getline; print; exit}' "$file" ;;
        testing)
            local companion="${file%.md}-testing.md"
            [ -f "$companion" ] || context_die "usage: testing view unavailable for $file"
            awk '/^## Automated tests$/{seen=1; next} seen && NF {print}' "$companion"
            ;;
        dependencies) awk 'tolower($0) ~ /depends on/ {print}' "$file" ;;
        ownership) sed -n '/^## Ownership$/,/^## Change target$/p' "$file" | sed '$d' ;;
        changed-documents) sed -n '1,80p' "$file" | grep -E '^(#|§ |[-*] )' | head -20 ;;
        validator) sed -n '/^## Acceptance criteria$/,/^## Handoff$/p' "$file" | head -30 ;;
        *) context_die "usage: unsupported view: $view" ;;
    esac
}

context_build_index() {
    local plan_dir="$1" output="$2"
    {
        local plan_file
        plan_file="$(context_resolve_document "$plan_dir" plan)"
        printf 'entry_id\tpath\tkind\thash\n'
        printf 'plan\t%s\tplan\t%s\n' "$plan_file" "$(context_hash_file "$plan_file")"
        find "$plan_dir" -type f -name 'goal.md' -not -path '*/context/*' | sort | while IFS= read -r file; do
            printf 'goal:%s\t%s\tgoal\t%s\n' "$(basename "$(dirname "$file")")" "$file" "$(context_hash_file "$file")"
        done
        find "$plan_dir" -type f -path '*/steps/*.md' -not -name '*-testing.md' -not -path '*/context/*' | sort | while IFS= read -r file; do
            goal="$(basename "$(dirname "$(dirname "$file")")")"; step="$(basename "$file" .md)"
            printf 'step:%s/%s\t%s\tstep\t%s\n' "$goal" "$step" "$file" "$(context_hash_file "$file")"
        done
        awk -F'|' 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+$/,"",v); return v} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {printf "%s\t%s/steps/%s.md\n",trim($2),trim($9),trim($10)}' "$plan_dir/work-unit-inventory.md" |
            while IFS=$'\t' read -r unit relative; do
                file="$plan_dir/$relative"
                [ -f "$file" ] || continue
                printf 'unit:%s\t%s\tunit\t%s\n' "$unit" "$file" "$(context_hash_file "$file")"
            done
        if [ -f "$plan_dir/work-unit-inventory.md" ]; then
            printf 'inventory\t%s\tinventory\t%s\n' "$plan_dir/work-unit-inventory.md" "$(context_hash_file "$plan_dir/work-unit-inventory.md")"
        fi
        if [ -f "$plan_dir/progress.md" ]; then
            printf 'progress\t%s\tprogress\t%s\n' "$plan_dir/progress.md" "$(context_hash_file "$plan_dir/progress.md")"
        fi
        if [ -f "$plan_dir/adversarial-review.md" ]; then
            printf 'adversarial-review\t%s\tadversarial-review\t%s\n' "$plan_dir/adversarial-review.md" "$(context_hash_file "$plan_dir/adversarial-review.md")"
        fi
        if [ -n "${CONTEXT_SOURCE_ROOT:-}" ] && [ -f "$CONTEXT_SOURCE_ROOT/planning/SKILL.md" ]; then
            printf 'source:SKILL.md\t%s\tsource\t%s\n' "$CONTEXT_SOURCE_ROOT/planning/SKILL.md" "$(context_hash_file "$CONTEXT_SOURCE_ROOT/planning/SKILL.md")"
            if [ -f "$CONTEXT_SOURCE_ROOT/planning/REVIEWER.md" ]; then
                printf 'source:REVIEWER.md\t%s\tsource\t%s\n' "$CONTEXT_SOURCE_ROOT/planning/REVIEWER.md" "$(context_hash_file "$CONTEXT_SOURCE_ROOT/planning/REVIEWER.md")"
            fi
        fi
    } > "$output"
}

context_write_json_result() {
    local command="$1" status="$2" generation="${3:-}" entry="${4:-}" error="${5:-}"
    local generation_json=null entry_json=null error_json=null
    [ -n "$generation" ] && generation_json="\"$generation\""
    [ -n "$entry" ] && entry_json="\"$entry\""
    [ -n "$error" ] && error_json="\"$error\""
    printf '{"command":"%s","status":"%s","snapshot_generation":%s,"entry_id":%s,"changed_ids":[],"affected_ids":[],"next_token":null,"error_code":%s}\n' \
        "$command" "$status" "$generation_json" "$entry_json" "$error_json"
}

context_publish_snapshot() {
    local plan_dir="$1" generation="$2" index="$3" manifest="$4" entries="$5"
    local target; target="$(context_snapshot_root "$plan_dir")/$generation"
    mkdir -p "$target/entries"
    cp "$index" "$target/index.tsv"
    cp "$manifest" "$target/manifest.tsv"
    # A source path ending in "/." is unspecified in POSIX and BSD cp has
    # historically differed, so copy the tree contents through tar instead.
    ( cd "$entries" && tar cf - . ) | ( cd "$target/entries" && tar xf - )
    printf '%s\n' "$generation" > "$target/READY"
    local current_tmp
    current_tmp="$(context_current_file "$plan_dir").tmp.$$"
    printf '%s\n' "$generation" > "$current_tmp"
    mv "$current_tmp" "$(context_current_file "$plan_dir")"
}

context_load_manifest() {
    local plan_dir="$1" current generation
    current="$(context_current_file "$plan_dir")"
    [ -s "$current" ] || context_die "stale: no current snapshot"
    generation="$(cat "$current")"
    [ -f "$(context_snapshot_root "$plan_dir")/$generation/READY" ] || context_die "stale: invalid current snapshot"
    printf '%s\n' "$generation"
}

context_register_processed_entry() {
    local plan_dir="$1" entry="$2" file hash state temporary
    file="$(context_resolve_document "$plan_dir" "$entry")"
    hash="$(context_hash_file "$file")"
    state="$(context_processed_file "$plan_dir")"
    mkdir -p "$(dirname "$state")"
    temporary="${state}.tmp.$$"
    awk -F'\t' -v wanted="$entry" '$1 != wanted {print}' "$state" 2>/dev/null > "$temporary" || :
    printf '%s\t%s\n' "$entry" "$hash" >> "$temporary"
    LC_ALL=C sort -t $'\t' -k1,1 "$temporary" > "$state"
    rm -f "$temporary"
}

context_changed_entries() {
    local plan_dir="$1" state entry old file current
    state="$(context_processed_file "$plan_dir")"
    [ -f "$state" ] || return 0
    while IFS=$'\t' read -r entry old; do
        [ -n "$entry" ] || continue
        file="$(context_resolve_document "$plan_dir" "$entry")"
        if [ ! -f "$file" ]; then
            printf '%s\tdeleted\n' "$entry"
            continue
        fi
        current="$(context_hash_file "$file")"
        [ "$current" = "$old" ] || printf '%s\t%s\n' "$entry" "$current"
    done < "$state"
}

context_audit_all() {
    local plan_dir="$1" generation="$2" current_index snapshot_index temporary
    current_index="$(mktemp "${TMPDIR:-/tmp}/context-index.XXXXXX")"
    temporary="${current_index}.snapshot"
    trap 'rm -f "$current_index" "$temporary"' RETURN
    context_build_index "$plan_dir" "$current_index"
    snapshot_index="$(context_snapshot_root "$plan_dir")/$generation/index.tsv"
    awk -F'\t' -v snapshot="$snapshot_index" '
        BEGIN { while ((getline line < snapshot) > 0) { split(line, f, "\t"); if (f[1] != "entry_id") old[f[1]]=f[4] } close(snapshot) }
        NR > 1 { if (!($1 in old)) print $1 "\tunprocessed"; else if ($4 != old[$1]) print $1 "\tchanged"; seen[$1]=1 }
        END { for (id in old) if (!(id in seen)) print id "\tdeleted" }
    ' "$current_index" | LC_ALL=C sort
    trap - RETURN
}

context_invalidate_after_mutation() {
    local plan_dir="$1" entry="${2:-plan}" marker temporary
    [ -d "$(context_root "$plan_dir")" ] || return 0
    marker="$(context_root "$plan_dir")/mutation-handoff"
    temporary="${marker}.tmp.$$"
    printf '%s\n' "$entry" > "$temporary"
    mv "$temporary" "$marker"
}

# Per-role reader composition. Composition is per-role only; there is no global
# rule, and unset ROLE_ID means no gate at all.
# ---- quoted: emitted fields, tab-separated ----
# 1 = plan-context applies (0/1)
# 2 = combined plan-read budget cap in bytes (0 = not applied)
# ---- end quoted ----
context_role_reader_composition() {
    local role="$1" applies=1 budget=32768
    case "$role" in
        installer|oracle|eve) applies=0; budget=0 ;;
        maintainer) applies=1; budget=32768 ;;
        *) applies=1; budget=32768 ;;
    esac
    printf '%s\t%s\n' "$applies" "$budget"
}

# Resolve a ROLE_ID token to its canonical id by sourcing role-context.sh and
# reusing its native resolve_id(): its ROLES=() array is the only persona
# registry, and a second parser here would drift from it.
context_roles_file="${PLANNING_ROLE_CONTEXT:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/role-context.sh}"
context_resolve_role_id() {
    local token="$1" resolved=""
    if [ -f "$context_roles_file" ]; then
        resolved="$(bash -c 'set -euo pipefail; [ -f "$1" ] && source "$1"; resolve_id "$2"' _ "$context_roles_file" "$token" 2>/dev/null || true)"
    fi
    [ -n "$resolved" ] && printf '%s\n' "$resolved" || printf 'UNKNOWN\n'
}

# context_role_gate <max_bytes_var> : with ROLE_ID set, enforce the per-role
# allow-list on every subcommand; unknown roles fail closed. Returns 0 on allow
# (capping the byte budget through $1), exits 64 on refuse.
context_role_gate() {
    local maxb="${1:-max_bytes}" role applies budget
    [ -n "${ROLE_ID:-}" ] || return 0
    role="$(context_resolve_role_id "$ROLE_ID")"
    if [ -z "$role" ] || [ "$role" = UNKNOWN ]; then
        printf 'usage: unknown ROLE_ID "%s"\n' "$ROLE_ID" >&2
        exit 64
    fi
    IFS=$'\t' read -r applies budget <<< "$(context_role_reader_composition "$role")"
    [ "$applies" -eq 1 ] || {
        printf 'usage: role %s does not allow the plan-context gate (reader allow-list)\n' "$role" >&2
        exit 64
    }
    if [ "$maxb" != "-" ]; then
        # Cap via indirect-expansion + printf -v (no eval): functionally the
        # min of the declared budget and the caller's max_bytes.
        printf -v "$maxb" '%s' "$(( ${!maxb} < budget ? ${!maxb} : budget ))"
    fi
}
