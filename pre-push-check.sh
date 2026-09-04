#!/usr/bin/env bash
# MODE: DEV
# pre-push-check - the per-change gates from MAINTAINER.md section 4 and the
# PR hygiene rules in AGENTS.md, in one command.
#
# Run it before every push. The change set is everything that differs from
# master - the branch's commits plus the worktree and the index - and the fast,
# mechanical gates are applied to that:
#   git diff --check        whitespace, in the worktree, the index and the
#                           branch's committed diff
#   bash -n                 every changed shell script
#   static shell gate       the changed scripts at warning severity, with -x so
#                           `source=` resolves from disk. CI lints the whole
#                           live set; see the note at that gate for why the
#                           two agree and where they cannot
#   cargo fmt --check +     each crate under src/ touched by the change
#   cargo test              (skipped with a note when no crate changed)
#   register soundness      TODO.json and BUGS.json through reg_findings, the
#                           shipped implementation: ids, statuses, severities,
#                           priorities, parents, timestamps, reproductions,
#                           mechanism-on-confirmed, verification-on-fixed
#                           (needs rjq on PATH)
#   npm package baseline    every pinned byte size in
#                           npm-package-baseline.tsv against the working tree.
#                           Not npm's file selection - the full
#                           test-npm-package.sh still owns that
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
# usage; 65 = not a git repository, or neither master nor an upstream resolves
# and there is no branch diff to check.

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
# worktree or the index, measured against master.
#
# It used to prefer the tracking upstream, which quietly emptied the change set
# the moment a branch was pushed: `base..HEAD` is nothing when base IS the
# branch, so every gate driven by changed() skipped. A push would report "no
# crates under src/ changed; rust gates skipped" on a branch that rewrote a
# crate — the gates went quiet exactly when the work was finished.
#
# The merge base rather than origin/master itself, so a master that has moved
# ahead does not show its own commits as part of this branch's diff.
base=""
base_label=""
for ref in origin/master master; do
    if git rev-parse --verify "$ref" >/dev/null 2>&1; then
        base="$(git merge-base "$ref" HEAD 2>/dev/null || printf '%s' "$ref")"
        base_label="$ref"
        break
    fi
done
if [ -z "$base" ]; then
    base="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)"
    base_label="$base"
fi

# ---- master is refreshed before anything is measured ------------------------
# Every gate below measures the change set against master. A master ref that is
# behind the remote therefore makes already-merged work look like this branch's,
# and the gates report on a change set that does not exist: a register commit
# someone else merged an hour ago reads as yours. So master is fetched first,
# and a failure to fetch is a failure of the check rather than a silent fallback
# to a stale answer -- an unreliable change set is worse than no answer, because
# it looks like an answer.
#
# PRE_PUSH_SKIP_FETCH=1 exists for two callers only: a test driving this script
# in a throwaway clone whose origin is a local path, and diagnosis with no
# network. It is not an escape hatch for a slow link -- retry instead, since a
# dropped fetch here is usually the wifi (see the retry-first rule).
if [ "${PRE_PUSH_SKIP_FETCH:-0}" = 1 ]; then
    note "PRE_PUSH_SKIP_FETCH=1: master not refreshed, the change set may be stale"
elif git rev-parse --git-dir >/dev/null 2>&1 && git remote get-url origin >/dev/null 2>&1; then
    if git fetch --quiet origin master 2>/dev/null; then
        ok "fetched origin master, so the change set is measured against it"
    else
        bad "could not fetch origin master; the change set would be measured against a stale ref"
        note "retry first -- a dropped fetch is usually the network, not the remote"
        note "to check an unrelated gate meanwhile: PRE_PUSH_SKIP_FETCH=1 ./pre-push-check.sh"
        printf 'pre-push-check: 1 failure(s) - master could not be refreshed\n'
        exit 1
    fi
else
    note "no origin remote; master cannot be refreshed and the change set may be stale"
fi

changed() { # <pathspec-filter...> -> changed files matching the filter
    { [ -n "$base" ] && git diff --name-only "$base..HEAD"; git diff --name-only; git diff --cached --name-only; } \
        2>/dev/null | sort -u | grep "$@" || true
}

printf 'pre-push-check (base: %s)\n' "${base_label:-no master or upstream; worktree only}"

# ---- 0. the registers live on their own branch ------------------------------
# BUGS.json and TODO.json are append-mostly arrays, so two branches that both
# file an entry both take the same next free id. Git does not see that: the
# additions land at different array positions, it merges them textually with NO
# conflict, and the result carries two unrelated entries under one id. That
# happened for real on 2026-09-04 -- eight duplicate ids in one merge, invisible
# until reg_findings ran -- and register-resolve.sh's advice in the textual case
# is to take one side, which silently drops the other's entries.
#
# The structural answer is a single writer: register entries are filed on the
# `bugs` branch and nowhere else, so ids are allocated in one place and no merge
# ever has to reconcile two sets of them.
#
# This fails immediately rather than at the end: the push is going to be
# refused, and there is no reason to spend three minutes of cargo tests first.
# `registers`, not `bugs`: a branch cannot be named `bugs` while any `bugs/*`
# branch exists -- git refuses a ref and a ref directory of the same name, and
# bugs/close-b95 is unmerged and checked out. Work branches use the `bug/`
# prefix, so `registers` cannot collide with them either.
register_branch=registers
register_changes="$(changed -E '^(BUGS|TODO)\.json$' || true)"
current_branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo HEAD)"
# PRE_PUSH_ALLOW_REGISTERS=1 is for transport, not authoring: the one-off
# transition that introduces this rule while register work is already in
# flight, and an integration branch that merges someone else's entries rather
# than filing its own. It must never be used to file an entry -- that is the
# whole thing being prevented, and the duplicate ids it produces surface as a
# merge nobody can resolve without reading two histories.
if [ "${PRE_PUSH_ALLOW_REGISTERS:-0}" = 1 ] && [ -n "$register_changes" ]; then
    note "PRE_PUSH_ALLOW_REGISTERS=1: register changes accepted off the $register_branch branch"
    register_changes=''
fi
if [ -n "$register_changes" ] && [ "$current_branch" != "$register_branch" ]; then
    bad "a register is modified outside the $register_branch branch"
    printf '%s\n' "$register_changes" | sed 's/^/    /'
    note "branch: $current_branch"
    note "THE TARGET BRANCH IS: $register_branch"
    note "  git switch $register_branch   (git switch -c $register_branch origin/master if it is not local yet)"
    note "  then file the entry with the shipped tools -- bin/<triple>/bugs add ... or"
    note "  bin/<triple>/todo add ... -- and push; the entry reaches master from there"
    note "a fix's resolution keys (fix, verification, status) go the same way, after the"
    note "  code lands, so the id is allocated and closed in one place"
    printf 'pre-push-check: 1 failure(s) - registers changed off the %s branch\n' \
        "$register_branch"
    exit 1
fi

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

# ---- 3. shellcheck on the scripts that differ from master ------------------
# CI lints every live script in one invocation, naming the generated libraries
# alongside them because a `source=` directive resolves only against files on
# the linter's own command line — omit them and every variable a sourcing
# script reads from them reports unassigned (SC2154).
#
# That invocation costs ~33s of a ~47s run and is paid in full whether the
# change touches one script or none. Here the change set is what matters, so
# only the scripts that differ from master are linted, with -x: shellcheck
# then follows `source=` from disk instead of requiring the target on the
# command line, which is what makes a per-file lint equivalent to the whole-set
# one. Measured over all 317 live scripts, linted one at a time: 2 disagree
# with the whole-set result without -x (plan-context-lib.sh,
# test-portable-helpers.sh — both unresolved-source false positives), 0 with
# it, and -x introduces no findings of its own. On this branch the gate drops
# from 32,676ms to 454ms.
#
# The libraries are still built first: -x resolves them from disk, so they have
# to exist. CI remains the authority on the full set — a change in one script
# can in principle provoke a finding in an unchanged script that sources it,
# and only the whole-set lint sees that.
if [ -x planning/scripts/build-plan-libs.sh ]; then
    planning/scripts/build-plan-libs.sh >/dev/null 2>&1 || true
fi
# Only scripts that still EXIST: a deletion is part of the change set, and
# feeding a deleted path to shellcheck fails with "openBinaryFile: does not
# exist" -- so removing a superseded script used to fail this gate, which is
# precisely the shape that teaches people to bypass a gate rather than use it.
changed_sh=""
for _sh in $(changed -E '\.sh$'); do
    [ -f "$_sh" ] && changed_sh="${changed_sh}${changed_sh:+
}$_sh"
done
unset _sh
if [ -z "$changed_sh" ]; then
    note "no shell scripts differ from ${base_label:-the base}; shellcheck skipped (CI lints all)"
elif command -v shellcheck >/dev/null 2>&1; then
    # shellcheck disable=SC2086
    if shellcheck -x -s bash --severity=warning $changed_sh >/dev/null 2>&1; then
        ok "shellcheck -x --severity=warning ($(printf '%s\n' "$changed_sh" | wc -l | tr -d ' ') changed vs ${base_label:-base})"
    else
        bad "shellcheck findings at warning severity (CI gates on these)"
        # shellcheck disable=SC2086
        shellcheck -x -s bash --severity=warning $changed_sh 2>&1 | sed -n '1,40p' >&2
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
# no row to disagree with. The full test remains authoritative for the file set.
#
# An absent file is NOT drift. Six baseline rows name generated, untracked
# artifacts (MAINTAINER.md 2.16) -- planning/REVIEWER.md and the five
# plan-*-lib.sh -- which `npm pack` builds in its prepack and a fresh checkout
# simply does not have. Counting those as failures made this gate red on a
# clean clone, which is the same shape of uselessness as a gate that can never
# fail: build-plan-libs.sh above produces five of them, and nothing here
# generates REVIEWER.md. They are reported as unchecked instead, so the count
# is visible rather than silently skipped.
baseline=planning/tests/fixtures/overview/npm-package-baseline.tsv
if [ -f "$baseline" ]; then
    unchecked=0
    drift="$(awk -F'\t' 'NR > 1 { print $1 "\t" $2 }' "$baseline" | while IFS="$(printf '\t')" read -r pkgpath size; do
        repopath="${pkgpath#package/}"
        [ -f "$repopath" ] || continue
        actual="$(wc -c < "$repopath" | tr -d ' ')"
        [ "$actual" = "$size" ] || printf '%s is %s bytes, baseline says %s; ' "$repopath" "$actual" "$size"
    done)"
    unchecked="$(awk -F'\t' 'NR > 1 { print $1 }' "$baseline" | while IFS= read -r pkgpath; do
        [ -f "${pkgpath#package/}" ] || printf 'x'
    done | wc -c | tr -d ' ')"
    baseline_rows=$(( $(wc -l < "$baseline") - 1 ))
    if [ -n "$drift" ]; then
        bad "npm package baseline drift: ${drift%; }"
        note "refresh the rows, then confirm with planning/tests/test-npm-package.sh"
    elif [ "$unchecked" -gt 0 ]; then
        ok "npm package baseline matches the tree ($((baseline_rows - unchecked)) of $baseline_rows files; $unchecked generated and not built here)"
    else
        ok "npm package baseline matches the tree ($baseline_rows files)"
    fi
else
    note "no npm package baseline at $baseline"
fi

# ---- 6. every tracked skill file is declared --------------------------------
# The baseline gate above checks SIZES of paths it already knows, deliberately:
# its header says npm's file SELECTION belongs to test-npm-package.sh. So a file
# ADDED to a skill directory and never declared in skill_files() passes here and
# fails in CI -- which is exactly what happened when the skill-length ratchet was
# added to planning/tests without a manifest row.
#
# --declarations-only is the half of that test which needs no built tree: it
# drops "every promised file exists on disk", which demands the 48 extensionless
# planning commands setup-dev-env.sh generates, and keeps the arm accounting that
# catches an undeclared file. The full form stays in the suite.
manifest_test="$repo_root/tests/test-skill-files-manifest.sh"
if [ -x "$manifest_test" ]; then
    if manifest_out="$("$manifest_test" --declarations-only 2>&1)"; then
        ok "every tracked skill file is declared in skill_files()"
    else
        bad "a skill file is tracked but not declared in skill_files()"
        printf '%s\n' "$manifest_out" | sed -n 's/^ *FAIL: /  /p'
        note "add it to the right arm in installer/src/50-manifest.sh, then installer/build.sh"
    fi
else
    note "no $manifest_test to check skill declarations with"
fi

# ---- 7. the whole suite, when asked for ------------------------------------
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
