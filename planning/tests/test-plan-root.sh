#!/usr/bin/env bash
# MODE: DEV
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=/dev/null
source "$repo_dir/planning/scripts/plan-document-lib.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

PLANS_ROOT="$tmp/custom-root"
resolved="$(plan_default_root)"
[ "$resolved" = "$tmp/custom-root" ]
plan_ensure_root_permissions "$resolved" "$repo_dir/planning/scripts" >/dev/null
[ -d "$resolved" ]

unset PLANS_ROOT
HOME="$tmp/home" resolved="$(plan_default_root)"
[ "$resolved" = "$tmp/home/.plans" ]
printf '%s\n' 'test-plan-root: PASS'
