#!/usr/bin/env bash
# MODE: DEV
# test-npm-package — compare npm pack output with the owned baseline.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
baseline="$repo_root/planning/tests/fixtures/overview/npm-package-baseline.tsv"
[ -f "$baseline" ] || { printf 'missing npm package baseline\n' >&2; exit 1; }

tmp="$(mktemp -d "${TMPDIR:-/tmp}/npm-package.XXXXXX")"
tarball=''
cleanup() { rm -rf "$tmp"; [ -z "$tarball" ] || rm -f "$repo_root/$tarball"; }
trap cleanup EXIT
tarball="$(cd "$repo_root" && npm pack --silent)"
[ -f "$repo_root/$tarball" ]
actual="$tmp/actual.tsv"
printf 'package_path\tbyte_size\n' >"$actual"
tar -tzf "$repo_root/$tarball" | LC_ALL=C sort | while IFS= read -r path; do
    size="$(tar -xOf "$repo_root/$tarball" "$path" | wc -c | tr -d ' ')"
    printf '%s\t%s\n' "$path" "$size"
done >>"$actual"
# The gzipped tarball's own size is deliberately NOT pinned: it varies with the
# zlib and npm version on the machine, so it fails on a runner whose toolchain
# differs from the one that wrote the baseline while the package is identical.
# What ships, and how big each file is, are both content-derived and portable.
cmp -s "$baseline" "$actual" || {
    printf 'npm package differs from owned baseline %s\n' "$baseline" >&2
    diff -u "$baseline" "$actual" >&2 || true
    exit 1
}

printf '%s\n' 'test-npm-package: PASS'
