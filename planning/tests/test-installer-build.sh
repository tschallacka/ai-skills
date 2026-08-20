#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
# test-installer-build — the committed install.sh is exactly what the build
# produces from installer/src/, so nobody hand-edits the artifact.
#
# Usage: test-installer-build.sh
#
# Four assertions:
#   1. installer/build.sh --check passes against the committed install.sh.
#   2. A one-byte change to install.sh makes --check fail (the guard has teeth).
#   3. Every part parses on its own.
#   4. The generated dependency block reached the artifact.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
builder="$repo_dir/installer/build.sh"
artifact="$repo_dir/install.sh"
src_dir="$repo_dir/installer/src"

note_fail() { printf 'installer-build: %s\n' "$1" >&2; t_record "$1"; }

[ -x "$builder" ] || { note_fail "missing or non-executable $builder"; exit 1; }
[ -d "$src_dir" ] || { note_fail "missing $src_dir"; exit 1; }

test_check_mode_clean() {
    if "$BASH" "$builder" --check >/dev/null; then
        printf '%s\n' 'test_check_mode_clean: PASS'
    else
        note_fail 'install.sh does not match installer/build.sh output'
        printf '%s\n' 'test_check_mode_clean: FAIL'
    fi
}

# Mutating the real artifact in place is the only faithful test of the guard, so
# it is restored from a copy taken first, on every exit path.
test_check_mode_detects_edit() {
    local saved status
    saved="$(mktemp "${TMPDIR:-/tmp}/install.sh.saved.XXXXXX")"
    cp "$artifact" "$saved"
    printf '# hand edit\n' >> "$artifact"
    status=0
    "$BASH" "$builder" --check >/dev/null 2>&1 || status=$?
    cp "$saved" "$artifact"
    rm -f "$saved"
    if [ "$status" -eq 1 ]; then
        printf '%s\n' 'test_check_mode_detects_edit: PASS'
    else
        note_fail "--check exited $status on a hand-edited install.sh, want 1"
        printf '%s\n' 'test_check_mode_detects_edit: FAIL'
    fi
}

test_parts_parse_and_cover() {
    local part parts=0 before
    before="$(t_failures)"
    for part in "$src_dir"/[0-9][0-9]-*.sh; do
        [ -f "$part" ] || { note_fail "no parts in $src_dir"; return; }
        parts=$((parts + 1))
        bash -n "$part" 2>/dev/null || note_fail "part does not parse in isolation: ${part##*/}"
    done
    [ "$parts" -ge 2 ] || note_fail "expected the installer to be split into parts, found $parts"
    if [ "$before" -eq "$(t_failures)" ]; then
        printf 'test_parts_parse_and_cover: PASS (%s parts)\n' "$parts"
    else
        printf '%s\n' 'test_parts_parse_and_cover: FAIL'
    fi
}

# The dependency tables must reach the artifact: an empty block would make every
# skill look requirement-free and the whole model a no-op.
test_dependency_block_generated() {
    local block
    block="$(awk '/^# BEGIN GENERATED DEPENDENCY BLOCK$/, /^# END GENERATED DEPENDENCY BLOCK$/' "$artifact")"
    case "$block" in
        *'runtime_requirements() {'*'runtime_requirement_strength() {'*'runtime_tool_install_hint() {'*)
            printf '%s\n' 'test_dependency_block_generated: PASS' ;;
        *)
            note_fail 'the generated dependency block is missing from install.sh'
            printf '%s\n' 'test_dependency_block_generated: FAIL' ;;
    esac
}

test_check_mode_clean
test_check_mode_detects_edit
test_parts_parse_and_cover
test_dependency_block_generated

exit "$(t_failures)"
