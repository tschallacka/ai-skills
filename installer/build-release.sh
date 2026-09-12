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
# marker against skill_files().
#
# Usage:
#   build-release.sh                  # write dist/ai-skills-<version>.tar.gz
#   build-release.sh --prepare        # stage host planning Rust commands only
#   build-release.sh --list           # print what would go in, one path per line
#   build-release.sh --out <dir>      # somewhere other than dist/
#   build-release.sh --help
#
# npm publishing is the same question with a different mechanism: package.json's
# files array ships whole directories, so .npmignore is generated from the same
# marker set by --npmignore.
#
# installer/src/05-config.sh and installer/src/50-manifest.sh are the two
# surviving fragments of the retired bash install.sh (see git history for the
# rest, and .agents/MAINTAINER.md for the retirement writeup): this script is
# now their only real consumer (a handful of tests source them too, for the
# same skill_files()/manifest cross-checks). They are not trimmed down to
# only what this script reads -- most of 05-config.sh is install.sh-picker
# state nothing here touches -- because the safe, low-risk move was keeping
# them intact and working rather than a from-scratch extraction under this
# same change. A future cleanup can narrow them once nothing else depends on
# the untouched parts either.

set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out_dir="$repo_root/dist"
mode=build

# The release collector and its pipeline use the same logical-to-physical path
# resolver install.sh once generated from this same file. Load it in the
# parent shell as well as in listed_by_installer(), because pipeline subshells
# do not inherit functions defined by a sibling subshell.
# shellcheck disable=SC1090
source "$repo_root/installer/src/05-config.sh"
# shellcheck disable=SC1090
source "$repo_root/installer/src/50-manifest.sh"
SOURCE_ROOT="$repo_root"

usage() {
    sed -n '3,24p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --list) mode=list ;;
        --npmignore) mode=npmignore ;;
        --prepare) mode=prepare ;;
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

host_target() {
    case "$(uname -s):$(uname -m)" in
        Linux:x86_64|Linux:amd64) printf '%s\n' x86_64-unknown-linux-musl ;;
        Linux:aarch64|Linux:arm64) printf '%s\n' aarch64-unknown-linux-musl ;;
        Darwin:x86_64) printf '%s\n' x86_64-apple-darwin ;;
        Darwin:arm64|Darwin:aarch64) printf '%s\n' aarch64-apple-darwin ;;
        MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64)
            printf '%s\n' x86_64-pc-windows-msvc ;;
        *) return 1 ;;
    esac
}

# Stage extensionless planning commands beside their shell oracles. The
# migration registry is the source of truth for the command-to-crate mapping;
# the renderer is deliberately omitted because another agent owns it. A root
# bin artifact from setup-dev-env.sh is reused, while a clean release build
# compiles the individual crate in the pinned target environment.
prepare_planning_rust_commands() {
    local target exe candidate artifact source
    target="$(host_target)" || {
        printf '%s: unsupported host for planning Rust commands\n' "${0##*/}" >&2
        return 66
    }
    exe=''
    case "$target" in *windows-msvc) exe='.exe' ;; esac
    while IFS= read -r candidate; do
        [ -n "$candidate" ] || continue
        [ -f "$repo_root/planning/scripts/$candidate.sh" ] || continue
        artifact="$repo_root/bin/$target/$candidate$exe"
        if [ ! -x "$artifact" ]; then
            source="$candidate"
            [ "$candidate" = overview-state ] && source=plan-overview
            [ -f "$repo_root/src/$source/Cargo.toml" ] || {
                # render-plans-board is intentionally absent from this branch.
                [ "$candidate" = render-plans-board ] && continue
                printf '%s: no crate for planning command %s\n' "${0##*/}" "$candidate" >&2
                return 66
            }
            command -v cargo >/dev/null 2>&1 || {
                printf '%s: cargo is required to build planning command %s\n' "${0##*/}" "$candidate" >&2
                return 66
            }
            ( cd "$repo_root" && cargo build --release \
                --manifest-path "$repo_root/src/$source/Cargo.toml" --target "$target" ) \
                || { printf '%s: cargo build %s failed\n' "${0##*/}" "$candidate" >&2; return 66; }
            artifact="$repo_root/target/$target/release/$candidate$exe"
        fi
        [ -x "$artifact" ] || {
            printf '%s: no executable artifact for planning command %s\n' "${0##*/}" "$candidate" >&2
            return 66
        }
        cp "$artifact" "$repo_root/planning/scripts/$candidate$exe"
        chmod +x "$repo_root/planning/scripts/$candidate$exe"
    done < <(awk -F '\t' '$2 == "runtime-binary" || $2 == "build-generator" { print $3 }' \
        "$repo_root/planning/rust-migration.tsv" | LC_ALL=C sort -u)
}

# A file's own header decides. Read the top only: a heredoc further down mentions
# these strings, and a test that plants one would otherwise ship itself.
declares_prod() { # <path>
    [ -f "$repo_root/$1" ] || return 1
    # A fixture's bytes are test input, not a declaration. A captured render
    # carries whatever marker the thing that produced it wrote, so reading it
    # as self-declaration ships the fixture. tests/test-mode-markers.sh exempts
    # the same paths for the same reason.
    case "$1" in */tests/fixtures/*) return 1 ;; esac
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
    local skill path source
    for skill in "${SKILL_NAMES[@]}"; do
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            source="$(source_file "$skill" "$path")"
            if [ ! -f "$source" ]; then
                case "$skill/$path" in
                    planning/bin/*/plan-overview|planning/bin/*/plan-overview.exe) continue ;;
                esac
            fi
            printf '%s/%s\n' "$skill" "$path"
        done <<EOF
$(skill_files "$skill")
EOF
    done
}

# The exact files src/installer/src/plugins.rs's own
# TUI_HINT_PLUGIN_CLAUDE_FILES/EDITOR_GATE_PLUGIN_FILES carry (the retired
# bash install.sh had a matching pair, installer/src/70-permissions.sh's
# tui_hint_plugin_claude_files/editor_gate_plugin_files, before it and
# install.sh were removed -- see git history).
#
# Neither existing mechanism above covers these: they are not a skill (no
# entry in skill_files()) and most of their files have no comment syntax a
# MODE marker could sit in (.json, .js). installer/build-release.sh's
# tarball is scoped to only what a marker or skill_files() actually lists,
# and installer/bootstrap.sh downloads exactly that tarball, so leaving
# these two plugin directories out of collect() silently degrades the Rust
# installer's tui-hint-plugin/editor-gate-plugin steps to a no-op with no
# error -- caught by running a real install from a real packaged release
# and finding neither plugin on disk despite "Installed: …" having printed.
tui_hint_plugin_files() {
    printf 'tui-hint-plugin/.claude-plugin/plugin.json\n'
    printf 'tui-hint-plugin/hooks/hooks.json\n'
    printf 'tui-hint-plugin/hooks/lib.sh\n'
    printf 'tui-hint-plugin/hooks/pre-tool-use.sh\n'
    printf 'tui-hint-plugin/opencode/tui-hint-plugin.js\n'
}

editor_gate_plugin_files() {
    printf 'editor-gate-plugin/.claude-plugin/plugin.json\n'
    printf 'editor-gate-plugin/hooks/hooks.json\n'
    printf 'editor-gate-plugin/hooks/lib.sh\n'
    printf 'editor-gate-plugin/hooks/editor-token\n'
    printf 'editor-gate-plugin/hooks/pre-tool-use-bash.sh\n'
    printf 'editor-gate-plugin/hooks/pre-tool-use-edit-write.sh\n'
}

collect() {
    {
        printf 'README.md\nLICENSE\npackage.json\n'
        tui_hint_plugin_files
        editor_gate_plugin_files
        local path
        while IFS= read -r path; do
            [ -n "$path" ] || continue
            declares_prod "$path" && printf '%s\n' "$path"
        done < <(cd "$repo_root" && git ls-files \
            planning project-specificies resource-limited-testing brainstorm \
            post-implementation-review todo bug-report)
        listed_by_installer
    } | while IFS= read -r path; do
        case "$path" in
            planning/scripts/*)
                printf 'planning/%s\n' "$(platform_relative_path planning "${path#planning/}")" ;;
            *) printf '%s\n' "$path" ;;
        esac
    done | sort -u
}

case "$mode" in
    prepare)
        prepare_planning_rust_commands
        ;;
    list)
        collect
        ;;
    npmignore)
        # Everything tracked that the release does not carry. Generated rather
        # than maintained: a new maintainer file would otherwise be published.
        { printf '# GENERATED FILE — do not edit. Written by installer/build-release.sh --npmignore\n'
          printf '# Everything not marked MODE: PROD. Regenerate after adding a file.\n'
          # The list below is derived from git ls-files, so it can only name
          # TRACKED files. Build scratch inside a directory package.json ships
          # wholesale is untracked, never appears there, and would be published
          # by any machine that had built — CI writes planning/.ci-bin/plan-crypt
          # and npm pack carried it. Untracked output needs a standing rule.
          printf '.ci-bin/\n'
          comm -13 <(collect) <(cd "$repo_root" && git ls-files | sort)
        }
        ;;
    build)
        prepare_planning_rust_commands \
            || { printf '%s: planning Rust command preparation failed\n' "${0##*/}" >&2; exit 66; }
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
        #
        # ALWAYS built with --target, even though every case below names the
        # very host this is running on: `uname` cannot tell a glibc host from
        # a musl one, so a plain `cargo build --release` here would link
        # against whatever libc the host's default toolchain happens to use
        # and land the result under a directory named for the musl triple
        # regardless -- the same bug installer/build-installer-release.sh's
        # own history already records and fixed for the installer binary.
        # host_target() -- the resolver this whole file already uses for
        # prepare_planning_rust_commands() -- names the target once so the
        # build and the destination directory can never disagree.
        if command -v cargo >/dev/null 2>&1; then
            chat_dir="$(host_target)" \
                || { printf '%s: unsupported host for chat binaries\n' "${0##*/}" >&2; exit 66; }
            ( cd "$repo_root" && cargo build --release --target "$chat_dir" --package chat-server-rs ) \
                || { printf '%s: cargo build chat-server-rs failed\n' "${0##*/}" >&2; exit 66; }
            ( cd "$repo_root" && cargo build --release --target "$chat_dir" --package chat-client-rs ) \
                || { printf '%s: cargo build chat-client-rs failed\n' "${0##*/}" >&2; exit 66; }
            ( cd "$repo_root" && cargo build --release --target "$chat_dir" --package chat-mcp ) \
                || { printf '%s: cargo build chat-mcp failed\n' "${0##*/}" >&2; exit 66; }
            mkdir -p "$repo_root/chat/bin/$chat_dir"
            chat_release="$repo_root/target/$chat_dir/release"
            cp "$chat_release/chat-server-rs" "$repo_root/chat/bin/$chat_dir/chat-server-rs"
            cp "$chat_release/chat-client-rs" "$repo_root/chat/bin/$chat_dir/chat-client-rs"
            cp "$chat_release/chat-mcp" "$repo_root/chat/bin/$chat_dir/chat-mcp"
        else
            # Prebuilt binaries must already be in place (CI build step).
            ls "$repo_root/chat/bin/"*/chat-server-rs >/dev/null 2>&1 \
                && ls "$repo_root/chat/bin/"*/chat-client-rs >/dev/null 2>&1 \
                && ls "$repo_root/chat/bin/"*/chat-mcp >/dev/null 2>&1 || {
                printf '%s: cargo not found and chat/bin binaries absent\n' "${0##*/}" >&2
                exit 66
            }
        fi
        # The two register binaries are release-built and never committed, the
        # same as the chat pair above, and skill_files() lists the host's row
        # for each -- so the copy loop below requires them for THIS host. Build
        # them when cargo is here and otherwise require the CI step's output,
        # rather than letting the loop fail with a bare "does not exist" on a
        # path nothing in this script ever writes. host_target(), not a fourth
        # copy of the case statement -- same --target reasoning as the chat
        # binaries above: uname cannot distinguish a glibc host from a musl
        # one, so the build and the destination directory it is named for must
        # come from the same resolved target or they can silently disagree.
        register_dir="$(host_target)" \
            || { printf '%s: unsupported host for the register binaries\n' "${0##*/}" >&2; exit 66; }
        register_exe=''
        case "$register_dir" in *windows*) register_exe='.exe' ;; esac
        for register_pair in 'bug-report:bugs' 'todo:todo'; do
            register_skill="${register_pair%%:*}"
            register_bin="${register_pair#*:}$register_exe"
            if [ -x "$repo_root/$register_skill/bin/$register_dir/$register_bin" ]; then
                continue
            elif command -v cargo >/dev/null 2>&1; then
                ( cd "$repo_root" && cargo build --release --target "$register_dir" \
                    --manifest-path "src/$register_skill/Cargo.toml" ) \
                    || { printf '%s: cargo build %s failed\n' "${0##*/}" "$register_skill" >&2; exit 66; }
                mkdir -p "$repo_root/$register_skill/bin/$register_dir"
                cp "$repo_root/target/$register_dir/release/$register_bin" \
                    "$repo_root/$register_skill/bin/$register_dir/$register_bin"
                chmod +x "$repo_root/$register_skill/bin/$register_dir/$register_bin"
            else
                printf '%s: no %s at %s/bin/%s/, and no cargo to build one\n' \
                    "${0##*/}" "$register_bin" "$register_skill" "$register_dir" >&2
                exit 66
            fi
        done
        # The compiled plan libraries are generated and never tracked
        # (MAINTAINER.md section 2.15), so a clean tree has none. Build-if-missing
        # here; staleness stays the tests' job. A listed file still missing after
        # this is the hard error below.
        libs_missing=0
        for lib in plan-core-lib.sh plan-crypt-lib.sh plan-document-lib.sh plan-progress-lib.sh plan-table-lib.sh; do
            [ -f "$repo_root/planning/scripts/$lib" ] || libs_missing=1
        done
        if [ "$libs_missing" -eq 1 ]; then
            "$repo_root/planning/scripts/build-plan-libs.sh" \
                || { printf '%s: build-plan-libs.sh failed\n' "${0##*/}" >&2; exit 66; }
        fi
        # REVIEWER.md is generated and never tracked (MAINTAINER.md section
        # 2.16); generation needs the compiled plan-crypt-lib.sh, so the library
        # step above must have run first. A present file is left alone - the
        # projection test owns staleness.
        if [ ! -f "$repo_root/planning/REVIEWER.md" ]; then
            "$repo_root/planning/scripts/generate-reviewer.sh" "$repo_root/planning" \
                || { printf '%s: generate-reviewer.sh failed\n' "${0##*/}" >&2; exit 66; }
        fi
        # The bundled rjq artifact is CI-delivered and never tracked (MAINTAINER.md
        # section 2.16). skill_files lists it platform-conditionally, so the
        # collect() check below requires it for THIS host: take a bootstrap-built
        # copy from the shared root bin/<triple> first, build from src/rjq when
        # cargo exists, and fail loudly when neither is there - a release cannot
        # ship the row silently missing.
        case "$(uname -s):$(uname -m)" in
            Linux:x86_64|Linux:amd64) rjq_dir=x86_64-unknown-linux-musl; rjq_bin=rjq ;;
            Linux:aarch64|Linux:arm64) rjq_dir=aarch64-unknown-linux-musl; rjq_bin=rjq ;;
            Darwin:x86_64) rjq_dir=x86_64-apple-darwin; rjq_bin=rjq ;;
            Darwin:arm64) rjq_dir=aarch64-apple-darwin; rjq_bin=rjq ;;
            MINGW*:x86_64|MSYS*:x86_64|CYGWIN*:x86_64|Windows*:x86_64|MINGW*:amd64|MSYS*:amd64|CYGWIN*:amd64|Windows*:amd64) rjq_dir=x86_64-pc-windows-msvc; rjq_bin=rjq.exe ;;
            *) rjq_dir='' ;;
        esac
        if [ -n "$rjq_dir" ] && [ ! -x "$repo_root/planning/bin/$rjq_dir/$rjq_bin" ]; then
            mkdir -p "$repo_root/planning/bin/$rjq_dir"
            if [ -x "$repo_root/bin/$rjq_dir/$rjq_bin" ]; then
                cp "$repo_root/bin/$rjq_dir/$rjq_bin" "$repo_root/planning/bin/$rjq_dir/$rjq_bin"
            elif command -v cargo >/dev/null 2>&1; then
                ( cd "$repo_root/src/rjq" && cargo build --release --target "$rjq_dir" ) \
                    || { printf '%s: cargo build rjq failed\n' "${0##*/}" >&2; exit 66; }
                cp "$repo_root/target/$rjq_dir/release/$rjq_bin" "$repo_root/planning/bin/$rjq_dir/$rjq_bin"
            else
                printf '%s: no rjq at planning/bin/%s/%s, no copy in bin/%s, and no cargo to build one\n' \
                    "${0##*/}" "$rjq_dir" "$rjq_bin" "$rjq_dir" >&2
                exit 66
            fi
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
