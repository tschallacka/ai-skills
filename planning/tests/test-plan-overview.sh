#!/usr/bin/env bash
# MODE: DEV
# Binary render contract: deterministic output, state extraction and deep links.
set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
binary="$repo/src/plan-overview/target/debug/plan-overview"
fixture="$repo/planning/tests/fixtures/overview/navigation"
cargo build --manifest-path "$repo/src/plan-overview/Cargo.toml" --offline >/dev/null
out="$(mktemp)"
trap 'rm -f "$out"' EXIT
OVERVIEW_NOW=fixed "$binary" --plan-dir "$fixture" --out "$out"
html="$(cat "$out")"
t_assert_contains 'binary renders HTML' '<!doctype html>' "$html"
t_assert_contains 'overview page is connected' 'Current phase' "$html"
t_assert_contains 'state is embedded' 'plan-state' "$html"
t_assert_contains 'goal content is rendered' 'Goals' "$html"
t_assert_contains 'unit content is reachable' 'Steps' "$html"
t_assert_eq 'no template tokens remain' 0 "$(grep -cE '@[A-Z_]+@' "$out" || true)"
t_assert_eq 'output is deterministic' "$html" "$(OVERVIEW_NOW=fixed "$binary" --plan-dir "$fixture" --out "$out"; cat "$out")"
rc=0; "$binary" --plan-dir "$repo/no-such-plan" --out "$out" >/dev/null 2>&1 || rc=$?
t_assert_eq 'missing plan is refused' 66 "$rc"
rc=0; "$binary" --bogus >/dev/null 2>&1 || rc=$?
t_assert_eq 'unknown option is refused' 64 "$rc"
t_end
