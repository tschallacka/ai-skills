#!/usr/bin/env bash
# MODE: DEV
# pre-push-check - the per-change gates from MAINTAINER.md section 4 and the
# PR hygiene rules in AGENTS.md, in one command.
#
# Run it before every push. It applies the fast, mechanical gates to the
# change about to be pushed:
#   git diff --check        whitespace, in the worktree, the index and the
#                           branch's committed diff
#   bash -n                 every changed shell script
#   static shell gate       every live script plus the generated libraries, at
#                           warning severity - the invocation CI gates on
#   cargo fmt --check +     each crate under src/ touched by the change
#   cargo test              (skipped with a note when no crate changed)
#   register soundness      TODO.json and BUGS.json parse, unique ids, known
#                           statuses (needs rjq on PATH)
# The registers update, the plan validator and the role-drift tests stay with
# MAINTAINER.md section 4: they need judgement about what changed, which a
# pre-push helper deliberately does not guess at.
#
# Usage:
#   pre-push-check.sh           the gates above
#   pre-push-check.sh --full    also run ./run-tests.sh (the whole suite,
#                               under the resource wrapper)
#   pre-push-check.sh --help
#
# Exit codes: 0 = every gate passed; 1 = at least one gate failed; 64 = bad
# usage; 65 = not a git repository or no upstream resolvable and no branch
# diff to check.

set -u
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
full=false

# Git runs hooks with the caller's environment, which may provide a different
# Cargo than the repository's pinned toolchain. Re-enter the flake once; the
# marker prevents recursion inside the development shell.
if [ -z "${AI_SKILLS_PREPUSH_IN_NIX:-}" ] && [ -z "${IN_NIX_SHELL:-}" ]; then
    command -v nix >/dev/null 2>&1 || {
        printf '%s: nix develop .#default is required for Rust pre-push checks\n' "${0##*/}" >&2
        exit 69
    }
    exec nix develop "$repo_root" --command env \
        AI_SKILLS_PREPUSH_IN_NIX=1 "$repo_root/pre-push-check.sh" "$@"
fi

usage() {
    awk 'NR > 1 && /^#/{ sub(/^# ?/, ""); print } /^set -u/{ exit }' "$0"
    exit "${1:-64}"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --full) full=true ;;
        -h|--help) usage 0 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; usage ;;
    esac
    shift
done

cd "$repo_root" || exit 65
git rev-parse --git-dir >/dev/null 2>&1 || {
    printf '%s: not a git repository\n' "${0##*/}" >&2
    exit 65
}

failures=0
ok() { printf '  ok    %s\n' "$1"; }
bad() { printf '  FAIL  %s\n' "$1"; failures=$((failures + 1)); }
note() { printf '  note  %s\n' "$1"; }

# The change set: committed work on this branch plus whatever is still in the
# worktree or the index. Upstream when set, master as the common fallback.
base=""
upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)"
if [ -n "$upstream" ]; then
    base="$upstream"
elif git rev-parse --verify origin/master >/dev/null 2>&1; then
    base="origin/master"
fi

changed() { # <pathspec-filter...> -> changed files matching the filter
    { [ -n "$base" ] && git diff --name-only "$base..HEAD"; git diff --name-only; git diff --cached --name-only; } \
        2>/dev/null | sort -u | grep "$@" || true
}

printf 'pre-push-check (base: %s)\n' "${base:-no upstream; worktree only}"

# ---- 1. whitespace ---------------------------------------------------------
ws_rc=0
git diff --check >/dev/null 2>&1 || ws_rc=1
git diff --cached --check >/dev/null 2>&1 || ws_rc=1
if [ -n "$base" ]; then
    git diff --check "$base..HEAD" >/dev/null 2>&1 || ws_rc=1
fi
if [ "$ws_rc" -eq 0 ]; then
    ok "git diff --check (worktree, index, branch diff)"
else
    bad "whitespace errors: git diff --check"
fi

# ---- 2. syntax on changed shell scripts ------------------------------------
sh_changed="$(changed '\.sh$')"
if [ -n "$sh_changed" ]; then
    syn_bad=0
    for f in $sh_changed; do
        [ -f "$f" ] || continue
        bash -n "$f" 2>/dev/null || { bad "bash -n: $f"; syn_bad=1; }
    done
    [ "$syn_bad" -eq 0 ] && ok "bash -n on changed scripts"
else
    note "no changed shell scripts; bash -n skipped"
fi

# ---- 3. shellcheck, the CI invocation --------------------------------------
# The generated libraries are untracked, so `git ls-files` cannot see them. A
# `source=` directive resolves only against files on the linter's own command
# line, so leaving them out reports every variable a sourcing script reads from
# them as unassigned. Build them, then name them (CONTRIBUTING.md).
if [ -x planning/scripts/build-plan-libs.sh ]; then
    planning/scripts/build-plan-libs.sh >/dev/null 2>&1 || true
fi
tracked_sh="$(
    { git ls-files '*.sh' | grep -v '^benchmark/results/'
      for lib in core crypt document progress table; do
          printf 'planning/scripts/plan-%s-lib.sh\n' "$lib"
      done
    } 2>/dev/null | LC_ALL=C sort -u || true
)"
if command -v shellcheck >/dev/null 2>&1; then
    # shellcheck disable=SC2086
    if shellcheck -s bash --severity=warning $tracked_sh >/dev/null 2>&1; then
        ok "shellcheck -s bash --severity=warning ($(printf '%s\n' "$tracked_sh" | wc -l | tr -d ' ') scripts)"
    else
        bad "shellcheck findings at warning severity (CI gates on these)"
    fi
else
    note "shellcheck not installed locally; CI still gates on it"
fi

# ---- 4. rust crates under src/ touched by the change -----------------------
crates="$(for f in $(changed -E '^src/[^/]+/'); do
    crate="${f#src/}"; crate="${crate%%/*}"
    [ -n "$crate" ] && printf '%s\n' "$crate"
done | LC_ALL=C sort -u)"
if [ -n "$crates" ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        note "src/ changed but cargo is not on PATH (nix develop); CI still runs fmt and test"
    else
        while IFS= read -r crate; do
            m="src/$crate/Cargo.toml"
            [ -f "$m" ] || continue
            if cargo fmt --check --manifest-path "$m" >/dev/null 2>&1; then
                ok "cargo fmt --check: $crate"
            else
                bad "cargo fmt --check: $crate (CI runs fmt before the build)"
            fi
            if cargo test --manifest-path "$m" >/dev/null 2>&1; then
                ok "cargo test: $crate"
            else
                bad "cargo test: $crate"
            fi
        done <<EOF
$crates
EOF
    fi
else
    note "no crates under src/ changed; rust gates skipped"
fi

# ---- 5. register soundness (needs rjq; skip quietly without it) ------------
# This delegates to reg_findings rather than carrying its own expression. The
# copy it used to carry was written in jq and used IN/1, which rjq -- the
# runtime the registers actually mandate -- cannot parse. It exited 5, the
# 2>/dev/null discarded the reason, the `|| true` discarded the status, and an
# empty findings string is the sound case: so the gate printed "ids unique,
# statuses known" for every register, always. Measured against a register with
# a duplicate id and two invalid statuses: no finding. Errors are now fatal to
# the check rather than silent, so a runtime that cannot run it says so.
if command -v rjq >/dev/null 2>&1; then
    # shellcheck source=planning/scripts/register-lib.sh
    source "$(cd "$(dirname "$0")/planning/scripts" && pwd)/register-lib.sh"
    for reg in TODO.json BUGS.json; do
        [ -f "$reg" ] || continue
        kind=bug
        [ "$reg" = TODO.json ] && kind=todo
        findings=''
        if ! findings="$(reg_findings "$kind" "$reg" 2>&1)"; then
            bad "$reg: the soundness check could not run: $findings"
        elif [ -n "$findings" ]; then
            bad "$reg: $findings"
        else
            ok "$reg is sound (reg_findings)"
        fi
    done
else
    note "rjq not on PATH; register soundness skipped (CI runs test-register-schemas)"
fi

# ---- 5b. the npm package baseline ------------------------------------------
# npm-package-baseline.tsv pins the byte size of every packaged file, so any
# edit to a shipped file fails planning/tests/test-npm-package.sh. That test
# runs `npm pack`, whose prepack builds the release tree and runs two suites --
# minutes, too slow for a pre-push gate. It does not need to be run: a packaged
# text file's bytes are the working tree's bytes, verified across all 153 rows.
# So the pinned sizes are checked directly here, in milliseconds.
#
# What this does NOT cover is npm's file *selection*: a newly shipped file has
# no row to disagree with. A row whose file has vanished is caught. The full
# test remains authoritative for the file set.
baseline=planning/tests/fixtures/overview/npm-package-baseline.tsv
if [ -f "$baseline" ]; then
    drift="$(awk -F'\t' 'NR > 1 { print $1 "\t" $2 }' "$baseline" | while IFS="$(printf '\t')" read -r pkgpath size; do
        repopath="${pkgpath#package/}"
        if [ ! -f "$repopath" ]; then
            printf '%s is in the baseline but not in the tree; ' "$repopath"
            continue
        fi
        actual="$(wc -c < "$repopath" | tr -d ' ')"
        [ "$actual" = "$size" ] || printf '%s is %s bytes, baseline says %s; ' "$repopath" "$actual" "$size"
    done)"
    if [ -n "$drift" ]; then
        bad "npm package baseline drift: ${drift%; }"
        note "refresh the rows, then confirm with planning/tests/test-npm-package.sh"
    else
        ok "npm package baseline matches the tree ($(( $(wc -l < "$baseline") - 1 )) files)"
    fi
else
    note "no npm package baseline at $baseline"
fi

# ---- 6. the whole suite, when asked for ------------------------------------
if [ "$full" = true ]; then
    if ./run-tests.sh; then
        ok "run-tests.sh: the whole suite"
    else
        bad "run-tests.sh reported failures"
    fi
else
    note "the whole deterministic suite is ./run-tests.sh (or re-run with --full)"
fi

printf 'pre-push-check: %s\n' "$([ "$failures" -eq 0 ] && echo PASS || echo "$failures failure(s)")"
[ "$failures" -eq 0 ]
