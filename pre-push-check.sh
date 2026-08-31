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
#   static shell gate       every tracked script, error severity - the same
#                           invocation CI gates on
#   cargo fmt --check +     each crate under src/ touched by the change
#   cargo test              (skipped with a note when no crate changed)
#   register soundness      TODO.json and BUGS.json parse, unique ids, known
#                           statuses (needs rjq on PATH)
# The registers update, the plan validator and the role-drift tests stay with
# MAINTAINER.md section 4: they need judgement about what changed, which a
# pre-push helper deliberately does not guess at.
#
# Usage:
#   pre-push-check.sh           # the gates above
#   pre-push-check.sh --full    # also run ./run-tests.sh (the whole suite,
#                               # under the resource wrapper)
#   pre-push-check.sh --help
#
# Exit codes: 0 = every gate passed; 1 = at least one gate failed; 64 = bad
# usage; 65 = not a git repository or no upstream resolvable and no branch
# diff to check.

set -u
export LC_ALL=C

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
full=false

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
[ "$ws_rc" -eq 0 ] && ok "git diff --check (worktree, index, branch diff)" \
    || bad "whitespace errors: git diff --check"

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
tracked_sh="$(git ls-files '*.sh' | grep -v '^benchmark/results/' || true)"
if command -v shellcheck >/dev/null 2>&1; then
    # shellcheck disable=SC2086
    if shellcheck -s bash --severity=error $tracked_sh >/dev/null 2>&1; then
        ok "shellcheck -s bash --severity=error ($(printf '%s' "$tracked_sh" | wc -l | tr -d ' ') scripts)"
    else
        bad "shellcheck findings at error severity (CI gates on these)"
    fi
else
    note "shellcheck not installed locally; CI still gates on it"
fi

# ---- 4. rust crates under src/ touched by the change -----------------------
crates=""
for f in $(changed '^src/[^/]+/'); do
    crate="${f#src/}"; crate="${crate%%/*}"
    case "$crate" in
        '') ;;
        *) case "
$crates" in *"|$crate
"*) ;; *) crates="$crates|$crate" ;; esac ;;
    esac
done
crates="${crates#|}"
if [ -n "$crates" ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        note "src/ changed but cargo is not on PATH (nix develop); CI still runs fmt and test"
    else
        for crate in $(printf '%s' "$crates" | tr '|' ' '); do
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
        done
    fi
else
    note "no crates under src/ changed; rust gates skipped"
fi

# ---- 5. register soundness (needs rjq; skip quietly without it) ------------
if command -v rjq >/dev/null 2>&1; then
    for reg in TODO.json BUGS.json; do
        [ -f "$reg" ] || continue
        findings="$(rjq -r '
          ([.tasks[]? .id] + [.bugs[]? .id]) as $ids
          | if ($ids | length) != ($ids | unique | length) then "duplicate ids" else empty end,
            (.tasks[]? | select(.status | IN("open","partly","blocked","decided","done","dropped","obsolete") | not)
                      | "\(.id) has an unknown status: \(.status)")' "$reg" 2>/dev/null || true)"
        if [ -n "$findings" ]; then
            bad "$reg: $findings"
        else
            ok "$reg parses, ids unique, statuses known"
        fi
    done
else
    note "rjq not on PATH; register soundness skipped (CI runs test-register-schemas)"
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
