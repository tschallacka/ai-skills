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
# The global root is the tsch-ai-skills XDG home. XDG_CONFIG_HOME is unset
# explicitly: inheriting the developer's own resolves outside $tmp, and the
# assertion would then describe their machine rather than the resolver.
resolved="$(env -u XDG_CONFIG_HOME -u PLANS_ROOT HOME="$tmp/home" "$BASH" -c '
    set -euo pipefail
    source "$1/planning/scripts/plan-document-lib.sh"
    plan_default_root
' _ "$repo_dir")"
[ "$resolved" = "$tmp/home/.config/tsch-ai-skills/plans" ]

# XDG_CONFIG_HOME wins over $HOME/.config when it is set.
resolved="$(env -u PLANS_ROOT XDG_CONFIG_HOME="$tmp/xdg" HOME="$tmp/home" "$BASH" -c '
    set -euo pipefail
    source "$1/planning/scripts/plan-document-lib.sh"
    plan_default_root
' _ "$repo_dir")"
[ "$resolved" = "$tmp/xdg/tsch-ai-skills/plans" ]
printf '%s\n' 'test-plan-root: PASS'
