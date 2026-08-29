#!/usr/bin/env bash
# test-rjq-active-references.sh - reject active jq references after migration.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
offenders="$(git -C "$repo_root" grep -n -w jq -- \
    ':!BUGS.json' ':!TODO.json' ':!benchmark/results/**' \
    ':!tests/test-rjq-active-references.sh' || true)"
offenders="$(printf '%s\n' "$offenders" | awk -F: '
    $0 ~ /(^|:)#/ || $0 ~ /:\/\// || $0 ~ /\.md:/ { next }
    NF { print }
')"
if [ -n "$offenders" ]; then
    printf '%s\n' "$offenders" >&2
    printf '%s\n' 'active reference test: jq remains outside historical evidence' >&2
    exit 1
fi

command -v rjq >/dev/null 2>&1 || {
    printf '%s\n' 'active reference test: rjq is not on PATH' >&2
    exit 1
}
printf '%s\n' 'active reference test: PASS'
