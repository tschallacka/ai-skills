#!/usr/bin/env bash
# MODE: DEV
# test-platform-selection — exercise installer platform normalization.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$repo_root/installer/src/20-runtime-tools.sh"

assert_target() {
    local os="$1" arch="$2" expected="$3" actual
    actual="$(PLAN_OVERVIEW_TEST_MODE=1 PLAN_OVERVIEW_TEST_OS="$os" \
        PLAN_OVERVIEW_TEST_ARCH="$arch" normalize_platform)"
    [ "$actual" = "$expected" ] || {
        printf 'platform %s:%s normalized to %s, expected %s\n' \
            "$os" "$arch" "$actual" "$expected" >&2
        return 1
    }
}

assert_target Linux x86_64 x86_64-unknown-linux-musl
assert_target Linux aarch64 aarch64-unknown-linux-musl
assert_target Darwin x86_64 x86_64-apple-darwin
assert_target Darwin arm64 aarch64-apple-darwin
assert_target Windows_NT AMD64 x86_64-pc-windows-msvc

selected="$(PLAN_OVERVIEW_TEST_MODE=1 PLAN_OVERVIEW_TEST_OS=Windows_NT \
    PLAN_OVERVIEW_TEST_ARCH=AMD64 plan_overview_selected_artifact)"
[ "$selected" = bin/x86_64-pc-windows-msvc/plan-overview.exe ]

unsupported="$(PLAN_OVERVIEW_TEST_MODE=1 PLAN_OVERVIEW_TEST_OS=Windows_NT \
    PLAN_OVERVIEW_TEST_ARCH=ARM64 plan_overview_selected_artifact 2>&1 || true)"
case "$unsupported" in
    *'Plan overview unavailable on this platform (Windows_NT:ARM64): no prebuilt artifact is available.'*) ;;
    *) printf 'missing unavailable-platform notice: %s\n' "$unsupported" >&2; exit 1 ;;
esac

printf '%s\n' 'test-platform-selection: PASS'
