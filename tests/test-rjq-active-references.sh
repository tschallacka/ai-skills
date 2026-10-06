#!/usr/bin/env bash
# MODE: DEV
# test-rjq-active-references.sh - keep rjq and jq in their own lanes.
#
# Two separate uses, two separate rules:
#
#   - Shipped code (the files the npm package carries) runs rjq at runtime, by
#     its shared-bin path. It never names a bare jq and never calls a bare rjq
#     that only resolves through PATH.
#   - Build and test code runs on the flake's dev shell, which provides jq. It
#     may name jq freely, and this test does not look at it.
#
# "Shipped" is the set the npm package baseline records (planning/tests/fixtures/
# overview/npm-package-baseline.tsv), the same record test-release-package checks
# against a real pack. Anything outside it is build or test environment.
#
# planning/tests/fixtures/** is frozen render evidence, and *.archive.json /
# *.back.json are prose the register tools moved; neither is code that runs.
# src/installer/src/requirements.rs is the one shipped place a jq reference is
# intentional: runtime_tool_verify() falls back to a system jq once the bundled
# and the shared rjq have both come up empty, and a_missing_rjq_falls_back_to_a_
# system_jq is the test proving that fallback works.
set -euo pipefail
export LC_ALL=C

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
baseline="$repo_root/planning/tests/fixtures/overview/npm-package-baseline.tsv"
[ -f "$baseline" ] || {
    printf '%s\n' "active reference test: no npm package baseline at $baseline" >&2
    exit 1
}

# The shipped paths: the baseline's package/ entries with the prefix stripped.
shipped=()
while IFS= read -r entry; do
    case "$entry" in
        package/*) shipped+=("${entry#package/}") ;;
    esac
done < <(cut -f1 "$baseline")
[ "${#shipped[@]}" -gt 0 ] || {
    printf '%s\n' 'active reference test: the baseline lists no shipped paths' >&2
    exit 1
}

# A bare jq word in shipped code is a dependency on a tool the package does not
# ship. Prose in comments and markdown is exempt; the exclusions name the frozen
# and intentional cases above.
offenders="$(git -C "$repo_root" grep -n -w jq -- "${shipped[@]}" \
    ':!planning/tests/fixtures/**' ':!*.archive.json' ':!*.back.json' \
    ':!src/installer/src/requirements.rs' || true)"
offenders="$(printf '%s\n' "$offenders" | awk -F: '
    # A comment is prose wherever it sits, indented or not (B154).
    $0 ~ /(^|:)[[:space:]]*#/ || $0 ~ /:[[:space:]]*\/\// || $0 ~ /\.md:/ { next }
    NF { print }
')"
if [ -n "$offenders" ]; then
    printf '%s\n' "$offenders" >&2
    printf '%s\n' 'active reference test: shipped code names jq; shipped code uses rjq by its shared-bin path' >&2
    exit 1
fi

# A bare rjq command in shipped code resolves only through PATH, which a shipped
# tool must never depend on. The shipped path is a variable holding a full path.
#   A command position: the start of a line, or after a pipe, a separator, a
#   backtick or a $( substitution, then rjq and an option or a quoted filter.
#   Prose ("with rjq the JSON"), a test name and a register row do not match.
bare="$(git -C "$repo_root" grep -n -E '(^|[|;&`(]|\$\()[[:space:]]*rjq[[:space:]]+(-|'\''|")' -- "${shipped[@]}" \
    ':!planning/tests/fixtures/**' ':!src/rjq/**' ':!*.tsv' || true)"
bare="$(printf '%s\n' "$bare" | awk -F: '
    $0 ~ /(^|:)[[:space:]]*#/ || $0 ~ /\.md:/ { next }
    NF { print }
')"
if [ -n "$bare" ]; then
    printf '%s\n' "$bare" >&2
    printf '%s\n' 'active reference test: shipped code calls a bare rjq; resolve it to the shared-bin path first' >&2
    exit 1
fi

# Looking rjq up on PATH is the same dependency by another route: shipped code
# resolves it through plan_rjq or the shared-bin path, never through command -v.
lookup="$(git -C "$repo_root" grep -n -E '(command -v|which|type)[[:space:]]+rjq([[:space:]]|$)' -- "${shipped[@]}" \
    ':!planning/tests/fixtures/**' ':!*.tsv' || true)"
lookup="$(printf '%s\n' "$lookup" | awk -F: '
    $0 ~ /(^|:)[[:space:]]*#/ || $0 ~ /\.md:/ { next }
    NF { print }
')"
if [ -n "$lookup" ]; then
    printf '%s\n' "$lookup" >&2
    printf '%s\n' 'active reference test: shipped code looks rjq up on PATH; resolve it to the shared-bin path instead' >&2
    exit 1
fi

# The shipped binaries are built from the Rust crates under src/, which the npm
# package does not carry, so their own bare rjq spawns are checked too. The
# rjq crate itself, the crates' tests, and the installer's documented fallback
# (requirements.rs) are the exceptions.
rust="$(git -C "$repo_root" grep -n -E 'Command::new\("rjq"\)|which\("rjq"\)' -- 'src/*/src/*.rs' 'src/*/src/**/*.rs' \
    ':!src/rjq/**' ':!src/installer/src/requirements.rs' ':!**/tests/**' || true)"
if [ -n "$rust" ]; then
    printf '%s\n' "$rust" >&2
    printf '%s\n' 'active reference test: a shipped binary spawns a bare rjq; resolve it with planning_register::rjq_program' >&2
    exit 1
fi

printf '%s\n' 'active reference test: PASS'
