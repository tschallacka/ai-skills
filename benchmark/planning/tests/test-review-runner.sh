#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if "$root/setup-and-run.sh" bad-name --iterative --fresh-review >/dev/null 2>&1; then
    echo 'duplicate review modes unexpectedly passed' >&2
    exit 1
fi
if "$root/setup-and-run.sh" bad-name --revisions= >/dev/null 2>&1; then
    echo 'empty revision list unexpectedly passed' >&2
    exit 1
fi
if "$root/setup-and-run.sh" bad-name --unknown >/dev/null 2>&1; then
    echo 'unknown option unexpectedly passed' >&2
    exit 1
fi
if "$root/run-benchmark.sh" bad-name /tmp --iterative --fresh-review >/dev/null 2>&1; then
    echo 'duplicate runner review modes unexpectedly passed' >&2
    exit 1
fi
if "$root/run-benchmark.sh" bad-name /tmp --revisions= >/dev/null 2>&1; then
    echo 'empty runner revision list unexpectedly passed' >&2
    exit 1
fi
printf 'Review runner option tests passed.\n'
