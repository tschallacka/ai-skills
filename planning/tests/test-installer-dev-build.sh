#!/usr/bin/env bash
# MODE: DEV
# test-installer-dev-build.sh — --dev-build prefers the repo-root dev build
# over a skill's shipped binaries.sh (T108).
#
# The bug this pins: install.sh only ever knew <skill>/bin/<triple>/, which
# only CI and a release populate. setup-dev-env.sh builds this host's crates
# into the repo-root bin/<triple>/ instead, so a plain install from a dev
# checkout silently installed whatever the shipped directory happened to
# hold -- a day-old binary in one real case, with no symptom beyond a guard
# that quietly stopped being accepted.
set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

triple=''
case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64)   triple=x86_64-unknown-linux-musl ;;
    Linux:aarch64|Linux:arm64)  triple=aarch64-unknown-linux-musl ;;
    Darwin:x86_64)              triple=x86_64-apple-darwin ;;
    Darwin:arm64)               triple=aarch64-apple-darwin ;;
esac
dev_bin="$repo_root/bin/$triple/bugs"
shipped_bin="$repo_root/bug-report/bin/$triple/bugs"
if [ -z "$triple" ] || [ ! -x "$dev_bin" ] || [ ! -x "$shipped_bin" ]; then
    printf '%s\n' 'test-installer-dev-build: SKIP (bugs is not built for this host; run ./setup-dev-env.sh)'
    exit 0
fi

work="$(mktemp -d "${TMPDIR:-/tmp}/installer-dev-build.XXXXXX")"
trap 'rm -rf "$work"; mv -f "$work.dev.bak" "$dev_bin" 2>/dev/null || true; mv -f "$work.shipped.bak" "$shipped_bin" 2>/dev/null || true' EXIT

# ---- 1. without --dev-build, the shipped binary is used, unconditionally ---
cp "$dev_bin" "$work.dev.bak"
cp "$shipped_bin" "$work.shipped.bak"
printf 'sentinel-dev-build' > "$dev_bin"
printf 'sentinel-shipped' > "$shipped_bin"
chmod +x "$dev_bin" "$shipped_bin"

t_a="$work/a"
( cd "$repo_root" && ./install.sh --skill bug-report --target "$t_a" --yes ) >/dev/null 2>&1 || true
[ "$(cat "$t_a/bug-report/bin/$triple/bugs" 2>/dev/null)" = 'sentinel-shipped' ] \
    || t_fail 'without --dev-build, the shipped binary was not what got installed'

# ---- 2. with --dev-build, the repo-root dev build wins, and is named -------
t_b="$work/b"
out="$( ( cd "$repo_root" && ./install.sh --skill bug-report --target "$t_b" --dev-build --yes ) 2>&1 || true )"
[ "$(cat "$t_b/bug-report/bin/$triple/bugs" 2>/dev/null)" = 'sentinel-dev-build' ] \
    || t_fail '--dev-build did not prefer the repo-root dev build'
case "$out" in
    *'dev build used for: bugs'*) : ;;
    *) t_fail "the summary did not name the binary that came from the dev build: $out" ;;
esac

# ---- 3. --dev-build refuses rather than silently installing nothing, when --
#         neither location has the binary --------------------------------
rm -f "$dev_bin" "$shipped_bin"
t_c="$work/c"
rc=0
( cd "$repo_root" && ./install.sh --skill bug-report --target "$t_c" --dev-build --yes ) \
    >"$work/refuse.out" 2>&1 || rc=$?
[ "$rc" -ne 0 ] || t_fail '--dev-build with no binary anywhere exited 0'
[ ! -e "$t_c/bug-report/bin/$triple/bugs" ] \
    || t_fail '--dev-build installed something despite refusing'
command grep -q -- '--dev-build' "$work/refuse.out" \
    || t_fail "the refusal did not name --dev-build: $(cat "$work/refuse.out")"

t_end 'test-installer-dev-build'
