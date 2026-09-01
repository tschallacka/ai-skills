#!/usr/bin/env bash
# MODE: DEV
# test-rjq-active-references.sh - reject active jq references after migration.
# planning/tests/fixtures/** is frozen render evidence: plan prose and rendered
# snapshots that record the migration itself, not call sites that execute jq.
# .npmignore is a generated inventory of paths; a fixture step file named
# 02-step-remove-jq-renderer.md puts the word in the list without any code
# calling jq.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
# The registers are prose about past work, and so is anything the register
# tools move that prose INTO: *.archive.json is what `todo prune` writes, and
# *.back.json what `migrate` writes before converting. Excluding them by shape
# rather than by name means the next dated archive does not turn this gate red.
offenders="$(git -C "$repo_root" grep -n -w jq -- \
    ':!BUGS.json' ':!TODO.json' ':!*.archive.json' ':!*.back.json' \
    ':!benchmark/results/**' \
    ':!tests/test-rjq-active-references.sh' ':!src/rjq/tests/differential.rs' \
    ':!planning/bin/**' ':!planning/tests/fixtures/**' ':!.npmignore' || true)"
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
    printf '%s\n' 'active reference test: rjq is not on PATH - run ./bootstrap.sh (builds it into the gitignored planning/bin path) or download it from the project releases page (queued as T70)' >&2
    exit 1
}
printf '%s\n' 'active reference test: PASS'
