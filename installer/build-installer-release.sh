#!/usr/bin/env bash
# MODE: DEV
# build-installer-release.sh — per-platform release tarballs for the Rust
# installer.
#
# installer/bootstrap.sh downloads ai-skills-<target-triple>.tar.gz and
# expects an executable `installer` sitting at the tarball root, alongside
# the same skill content install.sh's own release already carries. This
# script does not reimplement collecting that content: it builds the one
# universal tarball build-release.sh already knows how to assemble
# (running every prerequisite step that needs — the chat/register binaries,
# rjq, the plan libraries, REVIEWER.md), extracts it once, and then repacks
# that same tree once per target with that target's own installer binary
# added on top. The skill content is identical across every platform
# tarball; only the installer binary differs.
#
# Usage:
#   build-installer-release.sh                 # every target buildable or already built here
#   build-installer-release.sh --host-only      # only this host's own triple
#   build-installer-release.sh --target TRIPLE  # one explicit target
#   build-installer-release.sh --out DIR        # default: dist/
#   build-installer-release.sh --help
#
# A target this host cannot build (no matching Rust target installed, no
# cross linker) and has no prebuilt binary for is skipped with a note on
# stderr, not a hard failure — CI builds each target on its own runner and
# this script's job locally is "whatever this machine actually can produce."
# --target names one explicitly and DOES fail if that one cannot be built,
# since naming one is asking for exactly that artifact.

set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$repo_root/dist"
host_only=0
explicit_target=''

usage() {
    sed -n '3,22p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --host-only) host_only=1 ;;
        --target)
            [ "$#" -ge 2 ] || { printf '%s: --target needs a triple\n' "${0##*/}" >&2; usage; }
            explicit_target="$2"; shift ;;
        --out)
            [ "$#" -ge 2 ] || { printf '%s: --out needs a directory\n' "${0##*/}" >&2; usage; }
            out_dir="$2"; shift ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
    shift
done

# Matches installer-platform's Target::resolve and install.sh's
# normalize_platform. Windows is not in ALL_TARGETS below: bootstrap.sh is
# POSIX-only by construction (same reasoning as the interactive-shell
# skill's own PTY wrapper), so nothing downloads a Windows asset through it
# yet, and this script has nothing to gain from packing one no bootstrap
# path can reach.
host_target() {
    case "$(uname -s):$(uname -m)" in
        Linux:x86_64 | Linux:amd64) printf '%s\n' x86_64-unknown-linux-musl ;;
        Linux:aarch64 | Linux:arm64) printf '%s\n' aarch64-unknown-linux-musl ;;
        Darwin:x86_64) printf '%s\n' x86_64-apple-darwin ;;
        Darwin:arm64 | Darwin:aarch64) printf '%s\n' aarch64-apple-darwin ;;
        *) return 1 ;;
    esac
}

ALL_TARGETS=(x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-apple-darwin aarch64-apple-darwin)

targets_to_try() {
    if [ -n "$explicit_target" ]; then
        printf '%s\n' "$explicit_target"
        return 0
    fi
    if [ "$host_only" -eq 1 ]; then
        host_target
        return 0
    fi
    printf '%s\n' "${ALL_TARGETS[@]}"
}

installer_binary_path() { # <target> -> where its binary sits once built
    printf '%s/target/%s/release/installer\n' "$repo_root" "$1"
}

# Builds only when nothing is there yet, same "build-if-missing, staleness is
# the tests' job" posture build-release.sh already takes with the chat and
# register binaries. Returns 1 (not a hard exit) when cargo is absent or the
# build fails, so the caller can decide whether that is fatal.
#
# ALWAYS passes --target, even when it names the running host: `uname`
# cannot tell a glibc host from a musl one, so a "this is the host, skip
# --target" shortcut here would silently link against whatever libc the
# host's default toolchain happens to use and call the result
# x86_64-unknown-linux-musl regardless -- caught by inspecting a build this
# script itself produced with that shortcut still in place (`file` showed
# a glibc interpreter on a binary named as a musl one). The target's std
# library must be installed (`rustup target add`, the CI native job's own
# "Install the target standard library" step) for this to succeed.
ensure_installer_binary() {
    local target="$1" bin
    bin="$(installer_binary_path "$target")"
    [ -x "$bin" ] && return 0
    command -v cargo >/dev/null 2>&1 || return 1
    (cd "$repo_root" && cargo build --release -p installer --target "$target") || return 1
    [ -x "$bin" ]
}

# The shared skill payload, built once regardless of how many targets this
# run packs: build-release.sh's own "build" mode is what actually runs every
# prerequisite (chat/register binaries, rjq, the plan libraries,
# REVIEWER.md) rather than assuming a prior run already did.
universal_root=''
prepare_universal_payload() {
    [ -z "$universal_root" ] || return 0
    local universal_stage version_dir
    universal_stage="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills-universal.XXXXXX")"
    "$repo_root/installer/build-release.sh" --out "$universal_stage" \
        || { printf '%s: build-release.sh failed\n' "${0##*/}" >&2; return 66; }
    mkdir -p "$universal_stage/extracted"
    tar -xzf "$universal_stage"/*.tar.gz -C "$universal_stage/extracted"
    version_dir="$(find "$universal_stage/extracted" -mindepth 1 -maxdepth 1 -type d | head -1)"
    [ -n "$version_dir" ] || { printf '%s: universal payload extracted empty\n' "${0##*/}" >&2; return 66; }
    universal_root="$version_dir"
}

pack_target() {
    local target="$1" bin stage root tarball
    if ! ensure_installer_binary "$target"; then
        if [ -n "$explicit_target" ]; then
            printf '%s: cannot build or find an installer binary for %s\n' \
                "${0##*/}" "$target" >&2
            return 66
        fi
        printf '%s: skipping %s (no installer binary and cannot build one here)\n' \
            "${0##*/}" "$target" >&2
        return 0
    fi
    prepare_universal_payload || return $?
    bin="$(installer_binary_path "$target")"
    mkdir -p "$out_dir"
    stage="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills-installer-release.XXXXXX")"
    root="$stage/root"
    cp -R "$universal_root" "$root"
    cp "$bin" "$root/installer"
    chmod +x "$root/installer"
    find "$root" -type f -exec touch -t 202001010000 {} +
    tarball="$out_dir/ai-skills-$target.tar.gz"
    (cd "$root" && find . -type f | sed 's#^\./##' | LC_ALL=C sort \
        | tr '\n' '\0' | xargs -0 tar -cf - | gzip -n -9 >"$tarball")
    rm -rf "$stage"
    printf 'Wrote %s (%s)\n' "${tarball#"$repo_root"/}" "$(du -h "$tarball" | awk '{print $1}')"
}

# pack_target's own return already distinguishes "skip, nothing to build
# here" (0) from "real failure" (nonzero: an explicit --target that cannot
# be built, or the universal payload build itself failing, which is fatal
# regardless of how many targets were requested) -- so failing fast on the
# first nonzero return is correct in every case, not just the explicit one.
while IFS= read -r target; do
    [ -n "$target" ] || continue
    pack_target "$target"
done < <(targets_to_try)
