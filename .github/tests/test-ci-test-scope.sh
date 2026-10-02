#!/usr/bin/env bash
# MODE: DEV
# test-ci-test-scope.sh — ci-test-scope.sh (T116) decides which shell tests a
# CI run has to execute, so the property under test is the same one
# test-ci-scope.sh proves for crates: it only ever narrows when it has
# grounds, and every declared test's own marker is honoured exactly.
#
# Every case drives the change set through --files-from, which exists so
# these branches are reachable without inventing commits, matching
# test-ci-scope.sh's own convention.
set -uo pipefail
export LC_ALL=C

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$here/../.." && pwd)"
scope_sh="$here/../ci-test-scope.sh"
work="$(mktemp -d "${TMPDIR:-/tmp}/test-ci-test-scope.XXXXXX")"
trap 'rm -rf "$work"' EXIT
failures=0

# scope_for <file...> -> "<scope> <tests...>", the scope word then the tests
# line with its own "tests=" prefix stripped.
run_scope() { # <file...> -> sets SCOPE, REASON, LISTED (space-joined)
    local list="$work/files" out
    : > "$list"
    printf '%s\n' "$@" > "$list"
    out="$("$scope_sh" --files-from "$list")"
    SCOPE="$(printf '%s\n' "$out" | awk -F= '/^scope=/{print $2}')"
    REASON="$(printf '%s\n' "$out" | sed -n 's/^reason=//p')"
    LISTED="$(printf '%s\n' "$out" | sed -n 's/^tests=//p')"
}

check_scope() { # <label> <want> <file...>
    local label="$1" want="$2"; shift 2
    run_scope "$@"
    if [ "$SCOPE" = "$want" ]; then
        printf '  ok    %s\n' "$label"
    else
        printf '  FAIL  %s\n         want scope=%s, got scope=%s (%s)\n' \
            "$label" "$want" "$SCOPE" "$REASON"
        failures=$((failures + 1))
    fi
}

listed_has() { # <needle> -> 0 if LISTED contains it as a whole word
    case " $LISTED " in *" $1 "*) return 0 ;; esac
    return 1
}

echo "ci-test-scope: global inputs force a full run"
check_scope "the root manifest"        full Cargo.toml
check_scope "the toolchain file"       full rust-toolchain.toml
check_scope "the flake"                full flake.nix
check_scope "a workflow"               full .github/workflows/ci.yml
check_scope "the selector itself"      full .github/ci-test-scope.sh
check_scope "run-tests.sh"             full run-tests.sh
check_scope "lib-test.sh"              full planning/tests/lib-test.sh
# The selector must not exempt its OWN compiled source either -- matching
# test-ci-scope.sh's own equivalent case for src/ci-scope/* (goal 21).
check_scope "the compiled selector's own source" full src/ci-test-scope/src/main.rs

echo "ci-test-scope: a push to an integration branch is exhaustive"
for branch in master nextupdate; do
    got="$("$scope_sh" --push-to "$branch" | awk -F= '/^scope=/{print $2}')"
    if [ "$got" = "full" ]; then
        printf '  ok    a push to %s is full\n' "$branch"
    else
        printf '  FAIL  a push to %s must be full, got %s\n' "$branch" "$got"
        failures=$((failures + 1))
    fi
done

echo "ci-test-scope: nothing to diff against goes full, not selective-on-nothing"
check_scope "no files at all" full ""

echo "ci-test-scope: a huge change set is not trusted to selection"
big="$work/big"
: > "$big"
i=0
while [ "$i" -lt 101 ]; do
    printf 'docs/file-%s.md\n' "$i" >> "$big"
    i=$((i + 1))
done
run_scope_file() { "$scope_sh" --files-from "$big" | awk -F= '/^scope=/{print $2}'; }
got="$(run_scope_file)"
if [ "$got" = "full" ]; then
    printf '  ok    over 100 changed files goes full\n'
else
    printf '  FAIL  over 100 changed files should go full, got %s\n' "$got"
    failures=$((failures + 1))
fi

echo "ci-test-scope: a doc-only change narrows, never to empty"
# README.md hits none of the COVERS entries this file adds, so every declared
# test that names a source path is correctly excluded here -- the property
# under test is that the run is never trusted to an EMPTY list (an empty
# selective result falls back to full), not that nothing gets excluded.
check_scope "a doc-only change" selective README.md
if [ -n "$LISTED" ]; then
    printf '  ok    a doc-only change still selects a non-empty test set\n'
else
    printf '  FAIL  a doc-only change selected nothing: %s\n' "$REASON"
    failures=$((failures + 1))
fi

echo "ci-test-scope: a change hits the test that declares it, spares the rest"
# test-register-resolve-rebase-message.sh declares
# "src/bug-report/src/resolve.rs src/bug-report/src/main.rs"; a change there
# must keep it in and drop test-todo-resolve-rebase-message.sh, which
# declares the todo crate instead and shares none of this.
check_scope "a bug-report crate change" selective src/bug-report/src/resolve.rs
if listed_has tests/test-register-resolve-rebase-message.sh; then
    printf '  ok    the test that declares the changed file is kept\n'
else
    printf '  FAIL  the test declaring src/bug-report/src/resolve.rs was dropped: %s\n' "$LISTED"
    failures=$((failures + 1))
fi
if listed_has tests/test-todo-resolve-rebase-message.sh; then
    printf '  FAIL  an unrelated declared test was not excluded: %s\n' "$LISTED"
    failures=$((failures + 1))
else
    printf '  ok    an unrelated declared test is excluded\n'
fi

echo "ci-test-scope: a directory-prefix COVERS entry matches anything under it"
# T145 goal 25 (AR-115/AR-117): a repo-wide grep for '^# COVERS' found exactly
# two real, tracked files declaring a directory-prefix marker anywhere in the
# repo -- chat/tests/test-chat.sh and interactive-shell/tests/test-interactive-shell.sh
# -- and this goal retires both. Rather than depend on a real fixture that no
# longer exists, this scenario now mirrors
# src/ci-test-scope/tests/ci_test_scope_flow.rs's own already-passing unit
# test a_directory_prefix_covers_entry_matches_a_file_beneath_it: a synthetic
# scratch repo (a scratch chat/tests/test-chat.sh plus a stubbed
# run-tests.sh), spawning the real COMPILED ci-test-scope binary directly
# (bypassing this repo's own compiled-binary-preference wrapper, whose own
# exec always sets PLANNING_SKILL_ROOT to the real repo root first) with
# PLANNING_SKILL_ROOT pointed at the scratch dir instead. The bash fallback
# has no PLANNING_SKILL_ROOT support at all (src/ci-test-scope/src/repo_root.rs
# is compiled-binary-only), so this specific scenario -- like the Rust unit
# test it mirrors -- only exercises the compiled path; a missing compiled
# binary is a loud SKIP here, not a failure.
prefix_bin=""
if [ -x "$repo_root/bin/x86_64-unknown-linux-musl/ci-test-scope" ]; then
    prefix_bin="$repo_root/bin/x86_64-unknown-linux-musl/ci-test-scope"
elif [ -x "$repo_root/bin/aarch64-unknown-linux-musl/ci-test-scope" ]; then
    prefix_bin="$repo_root/bin/aarch64-unknown-linux-musl/ci-test-scope"
elif [ -x "$repo_root/bin/x86_64-apple-darwin/ci-test-scope" ]; then
    prefix_bin="$repo_root/bin/x86_64-apple-darwin/ci-test-scope"
elif [ -x "$repo_root/bin/aarch64-apple-darwin/ci-test-scope" ]; then
    prefix_bin="$repo_root/bin/aarch64-apple-darwin/ci-test-scope"
fi
if [ -z "$prefix_bin" ]; then
    printf '  SKIP  directory-prefix COVERS matching (no compiled ci-test-scope binary found; this scenario is compiled-binary-only)\n'
else
    prefix_work="$(mktemp -d "${TMPDIR:-/tmp}/test-ci-test-scope-prefix.XXXXXX")"
    mkdir -p "$prefix_work/chat/tests"
    printf '#!/usr/bin/env bash\n# COVERS: chat\necho hi\n' > "$prefix_work/chat/tests/test-chat.sh"
    cat > "$prefix_work/run-tests.sh" <<'RUNTESTSEOF'
#!/usr/bin/env bash
printf '%s\n' 'chat/tests/test-chat.sh'
RUNTESTSEOF
    printf 'chat/SKILL.md\n' > "$prefix_work/changed.txt"
    prefix_out="$(PLANNING_SKILL_ROOT="$prefix_work" env -u GITHUB_OUTPUT "$prefix_bin" \
        --files-from "$prefix_work/changed.txt")"
    prefix_scope="$(printf '%s\n' "$prefix_out" | awk -F= '/^scope=/{print $2}')"
    prefix_listed="$(printf '%s\n' "$prefix_out" | sed -n 's/^tests=//p')"
    rm -rf "$prefix_work"
    if [ "$prefix_scope" = selective ] && case " $prefix_listed " in *' chat/tests/test-chat.sh '*) true ;; *) false ;; esac; then
        printf '  ok    a directory COVERS entry matches a file beneath it\n'
    else
        printf '  FAIL  a synthetic chat/SKILL.md change should have hit the chat directory entry: scope=%s tests=%s\n' \
            "$prefix_scope" "$prefix_listed"
        failures=$((failures + 1))
    fi
fi

echo "ci-test-scope: an undeclared test always runs, whatever changed"
check_scope "an arbitrary unrelated change" selective docs/unrelated-file.md
# test-mode-markers.sh carries no COVERS marker as of this writing.
# PORTABILITY(pipefail-grep-q): grep -c drains the pipe instead of closing it
# on the first match, which under pipefail would otherwise report sed's own
# exit status as a SIGPIPE failure rather than the emptiness being tested.
if [ -f "$repo_root/tests/test-mode-markers.sh" ] \
    && ! sed -n '1,5{/^# COVERS: /p;}' "$repo_root/tests/test-mode-markers.sh" | grep -c . >/dev/null; then
    if listed_has tests/test-mode-markers.sh; then
        printf '  ok    an undeclared test is never excluded\n'
    else
        printf '  FAIL  an undeclared test was dropped: %s\n' "$LISTED"
        failures=$((failures + 1))
    fi
else
    printf '  note  tests/test-mode-markers.sh now carries a COVERS marker; picking another undeclared test is needed here\n'
fi

echo "ci-test-scope: usage"
if "$scope_sh" --nonsense >/dev/null 2>&1; then
    printf '  FAIL  an unknown flag should be rejected\n'
    failures=$((failures + 1))
else
    printf '  ok    an unknown flag is rejected\n'
fi

echo "ci-test-scope: no compiled binary falls back to the scope=full safe default"
# AR-100: never mutate the real, shared planning/scripts/plan-core-lib.sh in
# place -- copy ci-test-scope.sh into this test's own scratch work dir, whose
# planning/scripts/ has no plan-core-lib.sh, so the wiring's own
# [ -f .../plan-core-lib.sh ] check is false there with zero shared mutable
# state touched.
missing_binary_root="$work/missing-binary"
mkdir -p "$missing_binary_root/.github" "$missing_binary_root/planning/scripts"
cp "$scope_sh" "$missing_binary_root/.github/ci-test-scope.sh"
got="$(cd "$missing_binary_root" && ./.github/ci-test-scope.sh --files-from /dev/null)"
if [ "$got" = "scope=full
reason=ci-test-scope binary not found; run ./setup-dev-env.sh to build it
tests=" ]; then
    printf '  ok    a missing compiled binary falls back to scope=full\n'
else
    printf '  FAIL  a missing compiled binary should fall back to scope=full\n         got: %s\n' "$got"
    failures=$((failures + 1))
fi

echo
if [ "$failures" -eq 0 ]; then
    echo "test-ci-test-scope: PASS"
    exit 0
fi
printf 'test-ci-test-scope: FAIL (%s)\n' "$failures"
exit 1
