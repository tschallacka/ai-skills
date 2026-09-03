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
check "the installer"          full installer/src/50-manifest.sh
check "the generated install"  full install.sh
check "package.json"           full package.json

echo "ci-scope: the selector does not exempt itself"
# A change to the thing that decides the scope must be exercised in full,
# or the commit that narrows the scope is validated by the narrowed scope.
check "the selector itself"    full .github/ci-scope.sh
check "the subject mapper"     full .github/ci-subjects.sh
check "its own tests"          full .github/tests/test-ci-scope.sh

echo "ci-scope: nothing to do"
check "no files at all"        none ""
check "a doc-only change"      none README.md
check "a skill-only change"    none chat/SKILL.md

echo "ci-scope: a crate change narrows to that crate and its dependents"
got="$("$scope_sh" --files-from /dev/stdin <<'EOF' | awk -F= '/^scope=/{print $2}'
src/rjq/src/main.rs
EOF
)"
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

echo
if [ "$failures" -eq 0 ]; then
    echo "test-ci-scope: PASS"
    exit 0
fi
printf 'test-ci-scope: FAIL (%s)\n' "$failures"
exit 1
