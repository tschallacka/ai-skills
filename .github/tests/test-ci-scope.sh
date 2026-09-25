#!/usr/bin/env bash
# MODE: DEV
# test-ci-scope.sh — ci-scope.sh decides how much of the workspace CI builds,
# so the property under test is that it only ever narrows when it has grounds.
#
# Every case drives the change set through --files-from, which exists so these
# branches are reachable without inventing commits.
set -uo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
scope_sh="$here/../ci-scope.sh"
work="$(mktemp -d "${TMPDIR:-/tmp}/test-ci-scope.XXXXXX")"
trap 'rm -rf "$work"' EXIT
failures=0

# scope_for <file...> -> the scope word alone
scope_for() {
    local list="$work/files"
    : > "$list"
    printf '%s\n' "$@" > "$list"
    "$scope_sh" --files-from "$list" | awk -F= '/^scope=/{print $2}'
}

check() { # <label> <want> <file...>
    local label="$1" want="$2"; shift 2
    local got
    got="$(scope_for "$@")"
    if [ "$got" = "$want" ]; then
        printf '  ok    %s\n' "$label"
    else
        printf '  FAIL  %s\n         want scope=%s, got scope=%s\n' "$label" "$want" "$got"
        failures=$((failures + 1))
    fi
}

echo "ci-scope: global inputs force a full run"
check "the root manifest"      full Cargo.toml
check "the lock file"          full Cargo.lock
check "the toolchain file"     full rust-toolchain.toml
check "the flake"              full flake.nix
check "a workflow"             full .github/workflows/ci.yml

echo "ci-scope: installer/packaging-only changes need no crate rebuild (B300)"
# REGRESSION. None of these are Rust source, so none of them can change what a
# crate compiles to -- their own correctness is proved by dedicated jobs that
# run unconditionally regardless of this selector. Registering one new
# filename in installer/src/50-manifest.sh used to force scope=full on the
# ordinary act of shipping a new file, which is most commits.
check "the installer"          none installer/src/50-manifest.sh
check "the generated install"  none install.sh
check "package.json"           none package.json

echo "ci-scope: the selector does not exempt itself"
# A change to the thing that decides the scope must be exercised in full,
# or the commit that narrows the scope is validated by the narrowed scope.
check "the selector itself"    full .github/ci-scope.sh
check "the subject mapper"     full .github/ci-subjects.sh
check "its own tests"          full .github/tests/test-ci-scope.sh
# The same self-protection, extended for the compiled binary this selector
# now prefers: it lives under src/, not .github/, so the arm above alone
# does not cover it.
check "the compiled selector's own source" full src/ci-scope/src/main.rs

echo "ci-scope: a push to an integration branch is exhaustive"
# REGRESSION. On a push to master, HEAD is origin/master, so the merge base is
# HEAD and the diff is empty: the selector answered `scope=none`, every native
# leg was skipped, and master went green having compiled nothing. Run
# 33781612589 is that run -- 9 jobs, no native legs. Selection is a pull
# request feature; an integration branch does not get to narrow.
for branch in master nextupdate; do
    got="$("$scope_sh" --push-to "$branch" | awk -F= '/^scope=/{print $2}')"
    if [ "$got" = "full" ]; then
        printf '  ok    a push to %s is full\n' "$branch"
    else
        printf '  FAIL  a push to %s must be full, got %s\n' "$branch" "$got"
        failures=$((failures + 1))
    fi
done
# And it must win over an empty change set, which is the exact shape that bit:
# --files-from /dev/null is "nothing changed", and none would be the answer.
got="$("$scope_sh" --push-to master --files-from /dev/null | awk -F= '/^scope=/{print $2}')"
if [ "$got" = "full" ]; then
    printf '  ok    a push beats an empty change set\n'
else
    printf '  FAIL  a push must beat an empty change set, got %s\n' "$got"
    failures=$((failures + 1))
fi

echo "ci-scope: nothing to do"
check "no files at all"        none ""
check "a doc-only change"      none README.md
check "a skill-only change"    none chat/SKILL.md

echo "ci-scope: a crate change narrows to that crate and its dependents"
# The list goes through a file, not /dev/stdin: ci-scope is a native program,
# and on Windows the MSYS layer turns /dev/null into NUL for it but has no such
# translation for /dev/stdin, so the binary saw an unreadable path and (by
# design) fell back to scope=full.
leaf_list="$work/leaf-files"
printf '%s\n' 'src/rjq/src/main.rs' > "$leaf_list"
got="$("$scope_sh" --files-from "$leaf_list" | awk -F= '/^scope=/{print $2}')"
if [ "$got" = "selective" ]; then
    printf '  ok    a leaf crate is selective\n'
else
    printf '  FAIL  a leaf crate should be selective, got %s\n' "$got"
    failures=$((failures + 1))
fi

echo "ci-scope: a huge change set is not trusted to selection"
big="$work/big"
: > "$big"
i=0
while [ "$i" -lt 101 ]; do
    printf 'docs/file-%s.md\n' "$i" >> "$big"
    i=$((i + 1))
done
got="$("$scope_sh" --files-from "$big" | awk -F= '/^scope=/{print $2}')"
if [ "$got" = "full" ]; then
    printf '  ok    over 100 changed files goes full\n'
else
    printf '  FAIL  over 100 changed files should go full, got %s\n' "$got"
    failures=$((failures + 1))
fi

echo "ci-scope: an unusable threshold is discarded, never coerced"
# "abc" must not become 0 (every run full) nor a huge number (every run
# selective); it must fall back to the derived value and say so.
line="$("$scope_sh" --files-from /dev/null --threshold abc | awk -F= '/^reason=/{print $2}')"
printf '  note  reason with a junk threshold: %s\n' "$line"
if "$scope_sh" --files-from /dev/null --threshold abc >/dev/null 2>&1; then
    printf '  ok    a junk threshold still yields a decision\n'
else
    printf '  FAIL  a junk threshold should not crash the selector\n'
    failures=$((failures + 1))
fi

echo "ci-scope: usage"
if "$scope_sh" --nonsense >/dev/null 2>&1; then
    printf '  FAIL  an unknown flag should be rejected\n'
    failures=$((failures + 1))
else
    printf '  ok    an unknown flag is rejected\n'
fi

echo "ci-scope: no compiled binary falls back to the scope=full safe default"
# AR-100: never mutate the real, shared planning/scripts/plan-core-lib.sh in
# place -- copy ci-scope.sh into this test's own scratch work dir, whose
# planning/scripts/ has no plan-core-lib.sh, so the wiring's own
# [ -f .../plan-core-lib.sh ] check is false there with zero shared mutable
# state touched.
missing_binary_root="$work/missing-binary"
mkdir -p "$missing_binary_root/.github" "$missing_binary_root/planning/scripts"
cp "$scope_sh" "$missing_binary_root/.github/ci-scope.sh"
got="$(cd "$missing_binary_root" && ./.github/ci-scope.sh --files-from /dev/null)"
if grep -qF 'scope=full' <<<"$got" \
    && grep -qF 'reason=ci-scope binary not found; run ./setup-dev-env.sh to build it' <<<"$got"; then
    printf '  ok    a missing compiled binary falls back to scope=full\n'
else
    printf '  FAIL  a missing compiled binary should fall back to scope=full\n         got: %s\n' "$got"
    failures=$((failures + 1))
fi

echo
if [ "$failures" -eq 0 ]; then
    echo "test-ci-scope: PASS"
    exit 0
fi
printf 'test-ci-scope: FAIL (%s)\n' "$failures"
exit 1
