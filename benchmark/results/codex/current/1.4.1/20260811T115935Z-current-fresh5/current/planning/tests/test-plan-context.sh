#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
mode="${1:-default}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
if [ -z "${PLANNING_CONTEXT_CACHE:-}" ]; then
    printf '%s\n' 'test-plan-context: ERROR (PLANNING_CONTEXT_CACHE is required)' >&2
    exit 64
fi
legacy_fixture="$PLANNING_CONTEXT_CACHE"
if [ ! -d "$legacy_fixture" ]; then
    printf 'test-plan-context: ERROR (fixture unavailable: %s)\n' "$legacy_fixture" >&2
    exit 66
fi
cp -R "$legacy_fixture" "$tmp/plan"

materialize_fixture() {
    local fixture="$1" destination="$2" line relative current=''
    mkdir -p "$destination"
    while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in
            '@@ path='*)
                relative="${line#@@ path=}"
                case "$relative" in
                    /*|*..*|*[^A-Za-z0-9_./-]*) return 1 ;;
                esac
                current="$destination/$relative"
                mkdir -p "$(dirname "$current")"
                : > "$current"
                ;;
            *) [ -n "$current" ] || continue; printf '%s\n' "$line" >> "$current" ;;
        esac
    done < "$fixture"
}

run_benchmark() {
    local fixture_name fixture bench_dir unit output bytes baseline_bytes reduction rep helper_calls
    for fixture_name in small medium coupled; do
        fixture="$repo_dir/planning/tests/fixtures/context-cache-$fixture_name.md"
        bench_dir="$tmp/benchmark-$fixture_name"
        materialize_fixture "$fixture" "$bench_dir"
        "$repo_dir/planning/scripts/plan-context.sh" init --plan-dir "$bench_dir" >/dev/null
        unit="$(awk -F'|' '/^\|[[:space:]]*W[0-9]+[[:space:]]*\|/ {gsub(/^[[:space:]]+|[[:space:]]+/,"",$2); print $2; exit}' "$bench_dir/work-unit-inventory.md")"
        baseline_bytes=0
        while IFS= read -r file; do baseline_bytes=$((baseline_bytes + $(wc -c < "$file"))); done < <(find "$bench_dir" -type f -name '*.md' -not -path '*/context/*')
        bytes=0
        helper_calls=1
        for rep in 1 2 3 4 5; do
            output="$($repo_dir/planning/scripts/plan-context.sh read --plan-dir "$bench_dir" --unit "$unit" --view summary --read-only)"
            bytes=$((bytes + $(printf '%s' "$output" | wc -c | tr -d ' ')))
            helper_calls=$((helper_calls + 1))
        done
        bytes=$((bytes / 5))
        "$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$bench_dir" --changed >/dev/null
        helper_calls=$((helper_calls + 1))
        reduction=$(( (baseline_bytes - bytes) * 100 / baseline_bytes ))
        printf 'fixture=%s scenario=summary-read repetitions=5 baseline_bytes=%s output_bytes=%s reduction_pct=%s helper_calls=%s recommendation=%s\n' "$fixture_name" "$baseline_bytes" "$bytes" "$reduction" "$helper_calls" "$([ "$reduction" -gt 20 ] && printf continue || printf revise)"
    done
}

"$repo_dir/planning/scripts/plan-context.sh" init --plan-dir "$tmp/plan" >/dev/null
"$repo_dir/planning/scripts/plan-context.sh" read --plan-dir "$tmp/plan" --unit W37 --view summary --max-bytes 4096 >/dev/null
grep -q $'^unit:W37\t' "$tmp/plan/context/processed.tsv"
grep -q '^status=fresh$' <("$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$tmp/plan" --changed)
printf 'external edit\n' >> "$tmp/plan/05-portable-plan-root/steps/01-step-migrate-legacy-plans.md"
output="$("$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$tmp/plan" --changed)"
grep -q '^status=suspect$' <<< "$output"
grep -q '^changed_ids=unit:W37$' <<< "$output"
json="$("$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$tmp/plan" --entry unit:W37 --format json)"
grep -q '"status":"suspect"' <<< "$json"

case "$mode" in
    --cli-suite)
        if "$repo_dir/planning/scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document plan --unit W37 >/dev/null 2>&1; then exit 1; fi
        if "$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$tmp/plan" --changed --all >/dev/null 2>&1; then exit 1; fi
        if "$repo_dir/planning/scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document plan --max-bytes 0 >/dev/null 2>&1; then exit 1; fi
        printf '%s\n' 'test_cli_limits_and_errors: PASS'
        ;;
    --audit-suite|--audit-triggers)
        audit="$("$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$tmp/plan" --all)"
        grep -q '^status=suspect$' <<< "$audit"
        grep -q 'unit:W37' <<< "$audit"
        printf '%s\n' 'test_audit_scope_and_schema: PASS'
        ;;
    --fixture)
        fixture_name="${2:-small}"
        [ -f "$repo_dir/planning/tests/fixtures/context-cache-$fixture_name.md" ]
        printf 'fixture=%s\n' "$fixture_name"
        ;;
    --benchmark)
        run_benchmark
        ;;
esac
printf '%s\n' 'test-plan-context: PASS'
