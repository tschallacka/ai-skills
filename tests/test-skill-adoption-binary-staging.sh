#!/usr/bin/env bash
# MODE: DEV
# test-skill-adoption-binary-staging.sh - T2: the skill-adoption harness's
# prepare-cell.sh installs every skill directory as it finds it, so a
# dual-surface skill with no staged bin/<triple>/ binary used to ship a cell
# that could not run, with no warning. wa_require_staged_binaries (lib/common.sh)
# now refuses first. This drives it against a fake checkout: fully staged
# passes; one missing, or one not executable, refuses with 69 and names it.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$root/planning/tests/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/skill-adoption-staging.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# shellcheck source=benchmark/skill-adoption/lib/common.sh
source "$root/benchmark/skill-adoption/lib/common.sh"

fake="$work/checkout"
for pair in $WA_STAGED_BINARIES; do
    mkdir -p "$fake/${pair%%:*}/bin/test-triple"
    : > "$fake/${pair%%:*}/bin/test-triple/${pair#*:}"
    chmod +x "$fake/${pair%%:*}/bin/test-triple/${pair#*:}"
done

refused() {
    local rc=0
    wa_require_staged_binaries "$fake" 2>"$work/err" || rc=$?
    [ "$rc" -eq 69 ] || t_fail "expected 69 for $1, got $rc"
    grep -Fq "$1" "$work/err" || t_fail "the refusal does not name $1: $(cat "$work/err")"
    grep -Fq 'setup-dev-env.sh' "$work/err" || t_fail 'the refusal does not say how to stage'
}

wa_require_staged_binaries "$fake" 2>"$work/err" \
    || t_fail "a fully staged checkout was refused: $(cat "$work/err")"

rm "$fake/decisions/bin/test-triple/decisions-mcp"
refused 'decisions/bin/<triple>/decisions-mcp'
: > "$fake/decisions/bin/test-triple/decisions-mcp"
refused 'decisions/bin/<triple>/decisions-mcp'
chmod +x "$fake/decisions/bin/test-triple/decisions-mcp"

rm -rf "$fake/tailpipe/bin"
refused 'tailpipe/bin/<triple>/tailpipe-server-rs'

# prepare-cell.sh must ask before it creates a workspace or a registry line.
check_line="$(awk '/wa_require_staged_binaries "\$REPO_ROOT"/ {print NR; exit}' \
    "$root/benchmark/skill-adoption/prepare-cell.sh")"
results_line="$(awk '/^RESULTS_DIR=/ {print NR; exit}' "$root/benchmark/skill-adoption/prepare-cell.sh")"
[ -n "$check_line" ] || t_fail 'prepare-cell.sh never calls wa_require_staged_binaries'
[ -n "$check_line" ] && [ "$check_line" -lt "$results_line" ] \
    || t_fail 'prepare-cell.sh checks staging only after it has started creating files'

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'test-skill-adoption-binary-staging: PASS\n'
