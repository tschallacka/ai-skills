#!/usr/bin/env bash
# MODE: DEV
# test-installer-integration-carryover — an update with no --integration flag
# keeps the mode already installed, instead of silently reading as a request
# to switch back to `skill` (T109).
#
# The bug this pins: integration_mode_for fell back straight to
# INTEGRATION_DEFAULT ("skill") whenever no --integration was given, so a
# plain `install.sh --all` over an mcp install tore down the bridge binary and
# its agent registration on a run that only meant "give me the newest
# version". Measured for real on 2026-09-08: both --integration
# ai-text-editor=mcp and --integration chat=skill had to be passed by hand,
# from memory of what was already there, purely to stop an update
# unregistering a running editor bridge.
#
# Same isolated-source-tree-with-stub-binaries approach as
# test-installer-integration-mode.sh, and for the same reason: the filter
# decides on path, not content, so a fresh checkout with no bin/<triple>/
# still runs the whole test, and nothing here depends on this checkout's own
# (possibly stale or incomplete) shipped binaries.
set -euo pipefail
export LC_ALL=C
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/integration-carryover.XXXXXX")"
trap 'rm -rf "$work"' EXIT

case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64)   triple=x86_64-unknown-linux-musl ;;
    Linux:aarch64|Linux:arm64)  triple=aarch64-unknown-linux-musl ;;
    Darwin:x86_64)              triple=x86_64-apple-darwin ;;
    Darwin:arm64)               triple=aarch64-apple-darwin ;;
    MINGW*|MSYS*|CYGWIN*|Windows*) triple=x86_64-pc-windows-msvc ;;
    *)
        printf 'test-installer-integration-carryover: UNCONFIGURED (unknown host %s:%s)\n' \
            "$(uname -s)" "$(uname -m)" >&2
        exit 64
        ;;
esac
suffix=''
case "$triple" in x86_64-pc-windows-msvc) suffix='.exe' ;; esac

t_begin

source_root="$work/source"
mkdir -p "$source_root/planning" "$source_root/ai-text-editor/bin/$triple"
cp "$repo_dir/install.sh" "$source_root/install.sh"
printf '# stub: download_source() probes for this file to detect a checkout.\n' \
    > "$source_root/planning/SKILL.md"
t_copy_tree "$repo_dir/ai-text-editor" "$source_root/ai-text-editor"
for binary in ai-text-editor ai-text-editor-server ai-text-editor-mcp; do
    printf '#!/bin/sh\nexit 0\n' > "$source_root/ai-text-editor/bin/$triple/$binary$suffix"
    chmod +x "$source_root/ai-text-editor/bin/$triple/$binary$suffix"
done

installed_binaries() { # <target>
    find "$1" -type f -path '*/bin/*' 2>/dev/null \
        | sed -e 's|.*/||' \
        | LC_ALL=C sort \
        | tr '\n' ' '
}

target="$work/t"

install() { # <flag...> -> stdout+stderr, for the summary assertions
    ( cd "$source_root" && ./install.sh --skill ai-text-editor \
        --target "$target" --yes "$@" 2>&1 )
}

# ---- 1. an update with no flag keeps an mcp install mcp -------------------
out="$(install --integration mcp)"
case "$out" in *'integration mode: mcp (--integration)'*) : ;; *) t_fail "initial mcp install did not say so: $out" ;; esac
out="$(install)"
t_assert_eq 'a plain update keeps the bridge, not the direct client' \
    "$(installed_binaries "$target")" \
    "ai-text-editor-mcp$suffix ai-text-editor-server$suffix "
case "$out" in
    *'integration mode: mcp (carried forward from the existing install)'*) : ;;
    *) t_fail "the summary did not say the mode was carried forward: $out" ;;
esac

# ---- 2. the same, the other direction -------------------------------------
rm -rf "$target"
install --integration skill >/dev/null
out="$(install)"
t_assert_eq 'a plain update keeps the direct client, not the bridge' \
    "$(installed_binaries "$target")" \
    "ai-text-editor$suffix ai-text-editor-server$suffix "
case "$out" in
    *'integration mode: skill (carried forward from the existing install)'*) : ;;
    *) t_fail "the summary did not say skill mode was carried forward: $out" ;;
esac

# ---- 3. an explicit --integration still overrides the carried-forward mode -
out="$(install --integration mcp)"
t_assert_eq 'an explicit flag still switches an existing install' \
    "$(installed_binaries "$target")" \
    "ai-text-editor-mcp$suffix ai-text-editor-server$suffix "
case "$out" in
    *'integration mode: mcp (--integration)'*) : ;;
    *) t_fail "an explicit switch was not reported as explicit: $out" ;;
esac

# ---- 4. a genuine first install (nothing there yet) still gets the default -
rm -rf "$target"
out="$(install)"
t_assert_eq 'a first install with no flag still defaults to skill mode' \
    "$(installed_binaries "$target")" \
    "ai-text-editor$suffix ai-text-editor-server$suffix "
case "$out" in
    *'integration mode: skill (default, no prior install found)'*) : ;;
    *) t_fail "a first install was not reported as the default: $out" ;;
esac

# ---- 5. both modes' binaries present at once (a half-finished switch) is
#         reported, not silently resolved either way ------------------------
rm -rf "$target"
install --integration mcp >/dev/null
cp "$source_root/ai-text-editor/bin/$triple/ai-text-editor$suffix" \
    "$target/ai-text-editor/bin/$triple/ai-text-editor$suffix"
out="$(install 2>&1 || true)"
case "$out" in
    *'has binaries for both'*'modes'*) : ;;
    *) t_fail "a half-finished switch (both modes' binaries present) went unreported: $out" ;;
esac
# Reported exactly once, not once per internal call that happens to detect it.
count="$(printf '%s\n' "$out" | grep -c 'has binaries for both' || true)"
t_assert_eq 'the half-finished-switch warning is not printed twice for one decision' "$count" 1

t_end 'test-installer-integration-carryover'
