#!/usr/bin/env bash
# MODE: DEV
# test-installer-integration-mode — --integration must decide WHICH artifacts a
# skill installs, in both directions, and refuse a mode nothing offers.
#
# Usage: test-installer-integration-mode.sh
#
# Why this exists: the switch shipped untested. Measured by hand on 2026-09-04
# it worked, but nothing in the suite said so, so a refactor of the manifest
# could have collapsed the two modes into one and every other test would still
# have passed. An agent installing headlessly is relying on exactly this.
#
# It drives the GENERATED install.sh rather than sourcing a part file, because
# the mode tables are generated into the artifact and an empty table in a source
# part would make this pass while shipping nothing. The binaries are stubs: the
# filter decides on path, not content, so a fresh checkout with no bin/<triple>/
# still runs the whole test.
#
# This file is itself shipped, so it holds to the shipped-runtime dependency
# rule in CODE-STYLE.md §1: bash, POSIX coreutils, awk, sed, grep, git only.
# No python3.
set -euo pipefail
export LC_ALL=C
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/integration-mode.XXXXXX")"
trap 'rm -rf "$work"' EXIT

# The same host-to-triple mapping skill_files() uses, so the stubs land where
# the installer will look for them on this platform rather than only on Linux.
case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64)   triple=x86_64-unknown-linux-musl ;;
    Linux:aarch64|Linux:arm64)  triple=aarch64-unknown-linux-musl ;;
    Darwin:x86_64)              triple=x86_64-apple-darwin ;;
    Darwin:arm64)               triple=aarch64-apple-darwin ;;
    MINGW*|MSYS*|CYGWIN*|Windows*) triple=x86_64-pc-windows-msvc ;;
    *)
        printf 'test-installer-integration-mode: UNCONFIGURED (unknown host %s:%s)\n' \
            "$(uname -s)" "$(uname -m)" >&2
        exit 64
        ;;
esac
suffix=''
case "$triple" in x86_64-pc-windows-msvc) suffix='.exe' ;; esac

t_begin

# A source tree holding install.sh and the one skill under test.
#
# planning/SKILL.md is NOT optional scenery. download_source() decides between
# "local checkout" and "fetch the tarball" solely on whether that file sits
# beside install.sh, so a source root without it makes every install in this
# test download master from GitHub and assert against the published tree
# instead of the working one. That failure is quiet in the worst way: the
# installs succeed, the assertions compare the wrong thing, and the only
# symptom is that the test takes two minutes.
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

# Which artifacts landed, one basename per line, sorted.
installed_binaries() { # <target>
    find "$1" -type f -path '*/bin/*' 2>/dev/null \
        | sed -e 's|.*/||' \
        | LC_ALL=C sort \
        | tr '\n' ' '
}

install_with() { # <target-name> <flag...>
    local target="$work/$1"
    shift
    rm -rf "$target"
    ( cd "$source_root" && ./install.sh --skill ai-text-editor \
        --target "$target" --yes "$@" >/dev/null 2>&1 )
    installed_binaries "$target"
}

# ---- 1. mcp mode ships the bridge and the server it autostarts against --
#         not the skill client, which is the one artifact truly mode-exclusive
#         to `skill`. The server is unlisted in integration.tsv (installs in
#         every mode) rather than declared `skill`-only: both the client and
#         the bridge talk to it, and the bridge's own autostart spawns it
#         directly whenever none is running yet -- exactly the case an
#         mcp-only install without it could never recover from, confirmed by
#         hand against a real install (`cannot autostart ai-text-editor-server:
#         No such file or directory`).
t_assert_eq 'mcp mode installs the bridge and the server, not the skill client' \
    "$(install_with mcp --integration mcp)" \
    "ai-text-editor-mcp$suffix ai-text-editor-server$suffix "

# ---- 2. skill mode is the exact complement. Asserting only one direction is
#         how the socket work in this repo shipped a half-fix twice: a filter
#         that allowed everything would pass test 1 alone.
t_assert_eq 'skill mode installs the client and server, not the bridge' \
    "$(install_with skill --integration skill)" \
    "ai-text-editor$suffix ai-text-editor-server$suffix "

# ---- 3. The default is unchanged by the flag existing. An install that names
#         no mode must not quietly become an MCP install.
t_assert_eq 'no flag installs the direct client and server' \
    "$(install_with default)" \
    "ai-text-editor$suffix ai-text-editor-server$suffix "

# ---- 4. The per-skill form and the legacy alias reach the same decision, so
#         scripts passing the old flag keep working.
t_assert_eq 'the per-skill form selects mcp' \
    "$(install_with perskill --integration ai-text-editor=mcp)" \
    "ai-text-editor-mcp$suffix ai-text-editor-server$suffix "
t_assert_eq '--editor-integration is still honoured' \
    "$(install_with legacy --editor-integration mcp)" \
    "ai-text-editor-mcp$suffix ai-text-editor-server$suffix "

# ---- 5. Everything that is not an artifact is mode-free, so an mcp install is
#         a whole skill directory and not a lone binary.
for path in SKILL.md requires.tsv integration.tsv; do
    t_assert_eq "mcp mode still installs $path" \
        "$([ -f "$work/mcp/ai-text-editor/$path" ] && echo present || echo missing)" present
done

# ---- 6. A mode nothing offers is refused with exit 64, not silently defaulted.
#         CODE-CONTRACTS.md §12: 64 is "you invoked this wrong", and an
#         unattended run has only the status to go on.
t_expect_exit 64 'an undeclared mode is refused' \
    "$source_root/install.sh" --skill ai-text-editor --integration bogus \
    --target "$work/refused" --yes
t_expect_exit 64 'an undeclared mode is refused per skill' \
    "$source_root/install.sh" --skill ai-text-editor --integration ai-text-editor=bogus \
    --target "$work/refused" --yes
# ---- 7. Naming a skill that offers no choice is a mistake about the tool, so
#         it is refused rather than accepted as a preference. chat has no MCP
#         server at all today -- TODO.json T90 is the request for one.
t_expect_exit 64 'a skill with no modes refuses the flag' \
    "$source_root/install.sh" --skill chat --integration chat=mcp \
    --target "$work/refused" --yes

t_end
