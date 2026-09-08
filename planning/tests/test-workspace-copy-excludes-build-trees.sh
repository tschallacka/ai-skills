#!/usr/bin/env bash
# MODE: DEV
# test-workspace-copy-excludes-build-trees — the benchmark's published workspace
# copy leaves build artifacts behind, and honours TMPDIR for its scratch.
#
# B243: copy_workspace_for_publication() excluded only .env, so it copied the
# Rust `target/` tree — gigabytes, into what is tmpfs on a Linux workstation,
# and enough to take the host down. CI cannot see it: a fresh checkout has
# nothing built to copy, so this test builds the condition rather than waiting
# to meet it.
#
# Both spellings matter. The Rust workspace has a top-level `target/` and seven
# more under `src/*/`, so an anchored pattern alone would miss the nested ones —
# this asserts both.
#
# The TMPDIR half is the other half of the same defect: the test that called this
# helper hardcoded its destination under /tmp, so no export could move the copy
# to real disk. Sockets belong in /tmp; a multi-gigabyte workspace does not.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

setup="$repo_root/benchmark/planning/setup-benchmark.sh"
[ -f "$setup" ] || { echo "missing $setup" >&2; exit 66; }

work="$T_TMPDIR/copy"
mkdir -p "$work/src/fake-crate/target/debug" "$work/target/debug/deps" \
         "$work/node_modules/pkg" "$work/planning" "$work/.git"

# The artifacts a warm build leaves, plus the payload that must survive.
: > "$work/target/debug/some-binary"
: > "$work/target/debug/deps/some-dep-abc123"
: > "$work/src/fake-crate/target/debug/nested-binary"
: > "$work/node_modules/pkg/index.js"
printf 'secret\n' > "$work/.env"
printf 'payload\n' > "$work/planning/SKILL.md"
printf 'ref\n' > "$work/.git/HEAD"

# Drive the real helper rather than a copy of its tar line, so the assertions
# track the shipped exclusion list.
dest="$T_TMPDIR/published"
(
    # shellcheck disable=SC1090
    source "$setup" >/dev/null 2>&1 || true
    copy_workspace_for_publication "$work" "$dest"
) 2>/dev/null || {
    # setup-benchmark.sh runs work at source time; extract the one function
    # instead when sourcing it whole is not viable.
    awk '/^copy_workspace_for_publication\(\)/,/^}/' "$setup" > "$T_TMPDIR/fn.sh"
    # shellcheck disable=SC1090
    source "$T_TMPDIR/fn.sh"
    copy_workspace_for_publication "$work" "$dest"
}

present() { [ -e "$dest/$1" ]; }

# What must NOT be copied. Each is a separate assertion so a partial exclusion
# list names which pattern is missing rather than failing as one lump.
for excluded in \
    target/debug/some-binary \
    target/debug/deps/some-dep-abc123 \
    src/fake-crate/target/debug/nested-binary \
    node_modules/pkg/index.js \
    .env
do
    ! present "$excluded" \
        || t_fail "the published copy excludes $excluded (it was copied)"
done

# What must survive: excluding build output must not cost the payload, and .git
# is deliberately kept because the published workspace is used as a git repo.
for kept in planning/SKILL.md .git/HEAD; do
    present "$kept" \
        || t_fail "the published copy keeps $kept (it is missing)"
done

# The call site that actually caused B243 is the `TAG = current` tar that
# populates SRC_ROOT — the measured path was .../source/target/debug/rjq, and
# SRC_ROOT is "$CASE_ROOT/source". It is an inline pipeline rather than a
# function, and reaching it means resolving a benchmark agent and building a
# capsule, so it is asserted textually here: the functional assertions above
# prove the PATTERNS work against real tar (including `*/target` catching a
# nested crate), and this proves that call site carries them. Stated plainly
# because a textual assertion is the weaker of the two.
src_tar="$(awk '/tar -C "\$REPO_ROOT"/,/tar -x -C "\$SRC_ROOT"/' "$setup")"
[ -n "$src_tar" ] || t_fail "found the SRC_ROOT tar in setup-benchmark.sh"
for pattern in "./target" "*/target" "./node_modules" "*/node_modules"; do
    case "$src_tar" in
        *"--exclude='$pattern'"*) ;;
        *) t_fail "the SRC_ROOT copy excludes $pattern (a warm build tree is copied without it)" ;;
    esac
done

# TMPDIR is honoured for test scratch, which is what lets an operator move a
# multi-gigabyte copy off a tmpfs /tmp. T_TMPDIR is created under TMPDIR by
# lib-test.sh, so its parent is the ambient value this test was invoked with.
case "$T_TMPDIR" in
    /tmp/*)
        # Only correct when TMPDIR was itself unset or /tmp-based.
        case "${T_AMBIENT_TMPDIR:-/tmp}" in
            /tmp*) ;;
            *) t_fail "T_TMPDIR ignored TMPDIR: root is $T_TMPDIR" ;;
        esac
        ;;
esac

# And the socket root stays short and separate, because chromium's singleton
# socket cannot live at a long path.
case "${T_SOCKET_TMPDIR:-}" in
    "") t_fail "lib-test.sh exports T_SOCKET_TMPDIR for socket-bearing paths" ;;
    *)
        [ "${#T_SOCKET_TMPDIR}" -le 20 ] \
            || t_fail "T_SOCKET_TMPDIR stays short for the 104-byte socket cap (got ${#T_SOCKET_TMPDIR} chars: $T_SOCKET_TMPDIR)"
        ;;
esac

t_end
