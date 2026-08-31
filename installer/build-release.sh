#!/usr/bin/env bash
# MODE: DEV
# build-release.sh — assemble the prod tarball a release ships.
#
# A GitHub source archive carries the whole repository: the per-function library
# sources, 70 test scripts, the fixtures, the maintainer documentation, the
# benchmark tree. An end user needs none of it, and the installer downloading it
# means every install pays for it. So a release carries an asset built here,
# holding only what is marked MODE: PROD.
#
# The markers are the manifest. Nothing is listed twice: a file ships because it
# says it ships, and tests/test-mode-markers.sh has already cross-checked every
# marker against skill_files(). install.sh is added on top because it is the
# entry point and is generated rather than tracked as a skill file.
#
# Usage:
#   build-release.sh                  # write dist/ai-skills-<version>.tar.gz
#   build-release.sh --list           # print what would go in, one path per line
#   build-release.sh --out <dir>      # somewhere other than dist/
#   build-release.sh --help
#
# npm publishing is the same question with a different mechanism: package.json's
# files array ships whole directories, so .npmignore is generated from the same
# marker set by --npmignore.

set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$repo_root/dist"
mode=build

usage() {
    sed -n '3,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --list) mode=list ;;
        --npmignore) mode=npmignore ;;
        --out)
            [ "$#" -ge 2 ] || { printf '%s: --out needs a directory\n' "${0##*/}" >&2; usage; }
            out_dir="$2"; shift ;;
        --out=*) out_dir="${1#--out=}" ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
    shift
done

version="$(sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    "$repo_root/package.json" | head -1)"
[ -n "$version" ] || { printf '%s: package.json states no version\n' "${0##*/}" >&2; exit 65; }

# A file's own header decides. Read the top only: a heredoc further down mentions
# these strings, and a test that plants one would otherwise ship itself.
declares_prod() { # <path>
    case "$(sed -n '1,25p' "$repo_root/$1" 2>/dev/null)" in
        *'# MODE: PROD'*|*'<!-- MODE: PROD -->'*) return 0 ;;
    esac
    return 1
}

# Files with no marker at all: fixtures, formats with no comment syntax, the
# generated data files. They ship when the skill that owns them does, which is
# what skill_files() already says, so they are taken from there rather than
# guessed at here.
listed_by_installer() {
    # shellcheck disable=SC1090
    source "$repo_root/installer/src/05-config.sh"
    # shellcheck disable=SC1090
    source "$repo_root/installer/src/50-manifest.sh"
    SOURCE_ROOT="$repo_root"
    local skill
    for skill in "${SKILL_NAMES[@]}"; do
        skill_files "$skill" | sed "s|^|$skill/|"
    done
}

collect() {
    {
        printf 'install.sh\ninstall-ui.sh\nREADME.md\nLICENSE\npackage.json\n'
        local path
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            declares_prod "$path" && printf '%s\n' "$path"
        done < <(cd "$repo_root" && git ls-files \
            planning project-specificies resource-limited-testing brainstorm \
            post-implementation-review todo bug-report)
        listed_by_installer
    } | sort -u
}

case "$mode" in
    list)
        collect
        ;;
    npmignore)
        # Everything tracked that the release does not carry. Generated rather
        # than maintained: a new maintainer file would otherwise be published.
        { printf '# GENERATED FILE — do not edit. Written by installer/build-release.sh --npmignore\n'
          printf '# Everything not marked MODE: PROD. Regenerate after adding a file.\n'
          comm -13 <(collect) <(cd "$repo_root" && git ls-files | sort)
        }
        ;;
    build)
        mkdir -p "$out_dir"
        stage="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills-release.XXXXXX")"
        trap 'rm -rf "$stage"' EXIT
        root="$stage/ai-skills-$version"
        mkdir -p "$root"
        # The chat rust binaries are release-built, never committed. Build them
        # now (cargo lives in the nix dev shell or CI) and place them under
        # chat/bin/<host-triple>/ so the collect() copy loop below finds them
        # (skill_files resolves the host's platform to one triple dir). If cargo
        # is absent the build fails loudly rather than producing an empty package.
        if command -v cargo >/dev/null 2>&1; then
            ( cd "$repo_root/src/chat-server-rs" && cargo build --release ) \
                || { printf '%s: cargo build chat-server-rs failed\n' "${0##*/}" >&2; exit 66; }
            ( cd "$repo_root/src/chat-client-rs" && cargo build --release ) \
                || { printf '%s: cargo build chat-client-rs failed\n' "${0##*/}" >&2; exit 66; }
            case "$(uname -s):$(uname -m)" in
                Linux:x86_64|Linux:amd64) chat_dir=x86_64-unknown-linux-musl ;;
                Linux:aarch64|Linux:arm64) chat_dir=aarch64-unknown-linux-musl ;;
                Darwin:x86_64) chat_dir=x86_64-apple-darwin ;;
                Darwin:arm64) chat_dir=aarch64-apple-darwin ;;
                MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64) chat_dir=x86_64-pc-windows-msvc ;;
                *) printf '%s: unsupported host for chat binaries\n' "${0##*/}" >&2; exit 66 ;;
            esac
            mkdir -p "$repo_root/chat/bin/$chat_dir"
            cp "$repo_root/src/chat-server-rs/target/release/chat-server-rs" "$repo_root/chat/bin/$chat_dir/chat-server-rs"
            cp "$repo_root/src/chat-client-rs/target/release/chat-client-rs" "$repo_root/chat/bin/$chat_dir/chat-client-rs"
        else
            # Prebuilt binaries must already be in place (CI build step).
            ls "$repo_root/chat/bin/"*/chat-server-rs >/dev/null 2>&1 \
                && ls "$repo_root/chat/bin/"*/chat-client-rs >/dev/null 2>&1 || {
                printf '%s: cargo not found and chat/bin binaries absent\n' "${0##*/}" >&2
                exit 66
            }
        fi
        count=0
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            [ -f "$repo_root/$path" ] || {
                printf '%s: %s is listed for the release but does not exist\n' "${0##*/}" "$path" >&2
                exit 66
            }
            mkdir -p "$root/$(dirname "$path")"
            cp "$repo_root/$path" "$root/$path"
            count=$((count + 1))
        done < <(collect)
        tarball="$out_dir/ai-skills-$version.tar.gz"
        # Deterministic, so two builds of one commit can be compared and a
        # published asset can be checked against a local build. Three sources of
        # noise, each removed portably rather than with GNU-only tar flags --
        # a maintainer on macOS has BSD tar:
        #
        #   entry order   the paths are passed explicitly, in sorted order,
        #                 because both tars archive in argument order
        #   timestamps    every staged file is stamped to one fixed time
        #   gzip mtime    gzip -n omits it; both GNU and BSD gzip take -n
        #
        # What this does not normalise is the uid/gid tar records, so byte
        # equality holds between builds on one machine, and file-for-file
        # equality holds anywhere. test-release-package.sh asserts both.
        find "$root" -type f -exec touch -t 202001010000 {} +
        ( cd "$stage" && find "ai-skills-$version" -type f | LC_ALL=C sort \
            | tr '\n' '\0' | xargs -0 tar -cf - | gzip -n -9 > "$tarball" )
        printf 'Wrote %s (%s files, %s)\n' "${tarball#"$repo_root"/}" "$count" \
            "$(du -h "$tarball" | awk '{print $1}')"
        ;;
esac
