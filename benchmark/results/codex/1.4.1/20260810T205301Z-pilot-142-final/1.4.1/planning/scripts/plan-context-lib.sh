#!/usr/bin/env bash
# Shared implementation for the bounded planning context cache.

set -euo pipefail

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
        sleep 0.1
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
        goal:*) printf '%s/%s/goal.md\n' "$plan_dir" "${document_id#goal:}" ;;
        step:*)
            local value="${document_id#step:}" goal="${value%%/*}" step="${value#*/}"
            [ "$goal" != "$value" ] && [ -n "$step" ] || context_die "usage: invalid step entry: $document_id"
            printf '%s/%s/steps/%s.md\n' "$plan_dir" "$goal" "$step"
            ;;
        unit:W*)
            local unit="${document_id#unit:}" row goal step
            row="$(awk -F'|' -v wanted="$unit" 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+/,"",v); return v} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {if(trim($2)==wanted) print trim($9) "\t" trim($10)}' "$plan_dir/work-unit-inventory.md")"
            IFS=$'\t' read -r goal step <<< "$row"
            [ -n "${goal:-}" ] && [ -n "${step:-}" ] || context_die "not-found: work unit $unit"
            printf '%s/%s/steps/%s.md\n' "$plan_dir" "$goal" "$step"
            ;;
        *) context_die "usage: unsupported entry id: $document_id" ;;
    esac
}

context_entry_id() {
    case "$1" in
        plan|goal:*|step:*|unit:W*) printf '%s\n' "$1" ;;
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
        awk -F'|' 'function trim(v){gsub(/^[[:space:]]+|[[:space:]]+/,"",v); return v} /^\|[[:space:]]*W[0-9][0-9]+[[:space:]]*\|/ {printf "%s\t%s/steps/%s.md\n",trim($2),trim($9),trim($10)}' "$plan_dir/work-unit-inventory.md" |
            while IFS=$'\t' read -r unit relative; do
                file="$plan_dir/$relative"
                [ -f "$file" ] || continue
                printf 'unit:%s\t%s\tunit\t%s\n' "$unit" "$file" "$(context_hash_file "$file")"
            done
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
    cp -R "$entries"/. "$target/entries/"
    printf '%s\n' "$generation" > "$target/READY"
    local current_tmp="$(context_current_file "$plan_dir").tmp.$$"
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
