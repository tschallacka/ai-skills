#!/usr/bin/env bash
# MODE: DEV
# bootstrap - build the dev tools this repo's suite and registers need.
#
# The compiled helpers are CI-delivered and never tracked (planning/MAINTAINER.md
# section 2.16), so a fresh checkout has none. setup-dev-env.sh is the canonical
# build (flake cargo, every crate, one bin/<triple> at the root); this script is
# the no-nix fallback the run-tests and prepack bootstraps use: it builds what
# those bootstraps need into the same shared root bin/<triple> plan_bin_dir finds.
#
# Usage:
#   bootstrap.sh                        # every arm (currently: rjq)
#   bootstrap.sh rjq                    # one arm
#   bootstrap.sh rjq --path-only        # print only the artifact's bin dir
#   bootstrap.sh --help
#
# The rjq arm is a no-op when rjq is already on PATH or already built at
# bin/<triple>/rjq. Without cargo it exits 69 and names the release
# download, so a missing toolchain is never a silent dead end. --path-only is
# for callers that prepend the dir themselves (run-tests.sh); informational
# lines go to stderr either way.

set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

triple() {
    case "$(uname -s):$(uname -m)" in
        Linux:x86_64|Linux:amd64) printf '%s\n' x86_64-unknown-linux-musl ;;
        Linux:aarch64|Linux:arm64) printf '%s\n' aarch64-unknown-linux-musl ;;
        Darwin:x86_64) printf '%s\n' x86_64-apple-darwin ;;
        Darwin:arm64) printf '%s\n' aarch64-apple-darwin ;;
        MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64)
            printf '%s\n' x86_64-pc-windows-msvc ;;
        *) return 1 ;;
    esac
}

binary_name() {
    case "$1" in
        x86_64-pc-windows-msvc) printf '%s\n' rjq.exe ;;
        *) printf '%s\n' rjq ;;
    esac
}

arm_rjq() {
    if command -v rjq >/dev/null 2>&1; then
        printf 'rjq: already on PATH (%s)\n' "$(command -v rjq)" >&2
        return 0
    fi
    local t b out
    if ! t="$(triple)"; then
        printf 'bootstrap: no rjq artifact matches this host (%s:%s)\n' "$(uname -s)" "$(uname -m)" >&2
        return 69
    fi
    b="$(binary_name "$t")"
    out="$repo_root/bin/$t/$b"
    if [ ! -x "$out" ]; then
        if ! command -v cargo >/dev/null 2>&1; then
            printf 'bootstrap: cargo is missing, so rjq cannot be built here.\n' >&2
            printf '  Download the artifact from https://github.com/tschallacka/ai-skills/releases (queued as T70),\n' >&2
            printf '  or install a Rust toolchain and re-run this script.\n' >&2
            return 69
        fi
        if command -v rustup >/dev/null 2>&1; then
            rustup target add "$t" >&2
        fi
        mkdir -p "$(dirname "$out")"
        cargo build --release --manifest-path "$repo_root/src/rjq/Cargo.toml" --target "$t" >&2
        cp "$repo_root/src/rjq/target/$t/release/$b" "$out"
        printf 'rjq: built %s\n' "$out" >&2
    else
        printf 'rjq: already built at %s\n' "$out" >&2
    fi
    if [ "$path_only" = 1 ]; then
        printf '%s\n' "$repo_root/bin/$t"
    else
        printf 'export PATH="%s:$PATH"\n' "$repo_root/bin/$t"
    fi
}

usage() {
    awk 'NR > 1 && /^#/{ sub(/^# ?/, ""); print } /^set -euo/{ exit }' "$0"
}

path_only=0
if [ "${2:-}" = "--path-only" ]; then
    path_only=1
    shift 2>/dev/null || true
fi

arm=""
path_only=0
for arg in "$@"; do
    case "$arg" in
        --path-only) path_only=1 ;;
        rjq|--all) [ -z "$arm" ] && arm="$arg" ;;
        -h|--help) usage; exit 0 ;;
        *) printf 'bootstrap: unknown argument: %s (see --help)\n' "$arg" >&2; exit 64 ;;
    esac
done

case "${arm:---all}" in
    --all|rjq)
        arm_rjq || exit $?
        ;;
esac
