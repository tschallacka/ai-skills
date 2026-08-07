#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
grep -q 'for file in "\$SOURCE_ROOT/planning/scripts/"\*\.sh' "$repo_dir/install.sh"
for name in plan-context.sh plan-context-lib.sh plan-document-lib.sh; do
    [ -f "$repo_dir/planning/scripts/$name" ] || {
        printf 'expected helper is missing: %s\n' "$name" >&2
        exit 1
    }
done
printf '%s\n' 'test-installer-manifest: PASS'
