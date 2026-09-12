#!/usr/bin/env bash
# MODE: DEV
# Deterministic, repeatable test runner for the whole repo.
#
# Runs every test under tests/, planning/tests/ and benchmark/planning/tests/ in a
# fixed sorted order, each under the resource-limited wrapper, and reports a
# stable summary. The order and per-test result are the same on every run on
# the same host, so CI or a maintainer sees identical output.
#
# Usage: run-tests.sh [--verbose] [--select-file FILE] [--shard I/N]
#        run-tests.sh --list-only
#
# --list-only        print the discovered shell tests and crates, one
#                    repo-relative path per line, sorted, then exit -- no lock,
#                    no bootstrap, nothing run. This is the canonical list
#                    .github/ci-test-scope.sh (T116) reads rather than keeping
#                    a second copy of the suites array that could drift.
# --select-file FILE  FILE holds one repo-relative test path or crate dir per
#                    line (the same shape --list-only prints); only listed
#                    items run, everything else is skipped. Absent: every
#                    discovered item runs, exactly today's behaviour --
#                    selection is opt-in, never a silent default.
# --shard I/N        run only item I of every N, 0-indexed, chosen by
#                    deterministic position in the SORTED, post-selection work
#                    list (shell tests then crates) -- not per-suite, so one
#                    shard is never planning/tests' 84% of the work while
#                    another idles. The same I/N against the same inputs picks
#                    the same items every time, so a CI failure on shard 2/4
#                    reproduces locally with the identical flag. Absent: every
#                    item runs, exactly today's behaviour.
#
# One run at a time, machine-wide: the runner holds /tmp/ai-skills-run-tests.lock
# and refuses to start (exit 75) while another run really holds it. A lock whose
# recorded pid is gone -- or belongs to something that is not a suite run -- is
# stale and gets reused, so a killed run never wedges the next one.
# AI_SKILLS_ALLOW_CONCURRENT=1 bypasses it. --list-only takes no lock at all --
# it runs nothing, so it cannot collide with anything the lock protects.
#
# --shard is a CI-only concept: each CI runner is its own machine, so N shards
# is N machines each holding their own lock. Locally the lock still serialises
# a whole (possibly --select-file'd or --shard'd) run against any other, which
# is correct -- see the one-verification-at-a-time hazard where a second
# concurrent run deletes the first's worktree.
#
# A failing test's full output is always printed — it is the only diagnostic the
# runner has, and truncating it to the last 20 lines hid the failing assertion.
# `--verbose` additionally prints the output of tests that passed.
#
# `set -uo pipefail` deliberately omits `-e` (the sanctioned exception in
# CODE-STYLE.md §2): a failing test must not abort the loop before the summary
# is printed. Each test's status is captured explicitly instead.
#
# Suite paths are resolved against the repo root, so the runner works from any
# working directory.
set -uo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
wrapper="$repo_root/resource-limited-testing/scripts/limited-run.sh"

# The cap protects a developer's machine, which a disposable single-job runner
# is not. It also enforces nothing on either macOS runner, and on Linux a
# systemd scope does not return until every process in its cgroup exits, so a
# leaked process becomes a hang. AI_SKILLS_RESOURCE_LIMIT forces either way.
case "${AI_SKILLS_RESOURCE_LIMIT:-}" in
    0) wrapper="" ;;
    1) ;;
    *) [ -n "${GITHUB_ACTIONS:-}" ] && wrapper="" ;;
esac

# No test may run unbounded: without this a hang consumes the whole leg and the
# only symptom is a missing result, with no name attached to it.
test_timeout_seconds="${AI_SKILLS_TEST_TIMEOUT:-600}"
timeout_cmd=""
if command -v timeout >/dev/null 2>&1; then
    timeout_cmd=timeout
else
    printf '%s: no timeout(1) here, so a hanging test will not be bounded\n' \
        "${0##*/}" >&2
fi
verbose=false
list_only=false
select_file=""
shard_index=""
shard_total=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --verbose) verbose=true; shift ;;
        --list-only) list_only=true; shift ;;
        --select-file)
            [ "$#" -ge 2 ] || { printf '%s: --select-file needs a path\n' "${0##*/}" >&2; exit 64; }
            select_file="$2"; shift 2 ;;
        --shard)
            [ "$#" -ge 2 ] || { printf '%s: --shard needs I/N\n' "${0##*/}" >&2; exit 64; }
            case "$2" in
                *[!0-9]*/*[!0-9]*|*[!0-9/]*|*/*/*|'') printf '%s: --shard wants I/N, both non-negative integers, got %s\n' "${0##*/}" "$2" >&2; exit 64 ;;
            esac
            shard_index="${2%%/*}"
            shard_total="${2#*/}"
            [ "$shard_total" -gt 0 ] || { printf '%s: --shard total must be at least 1\n' "${0##*/}" >&2; exit 64; }
            [ "$shard_index" -lt "$shard_total" ] || { printf '%s: --shard index %s is out of range for %s shards\n' "${0##*/}" "$shard_index" "$shard_total" >&2; exit 64; }
            shift 2 ;;
        *) printf '%s: unknown argument: %s\n' "${0##*/}" "$1" >&2; exit 64 ;;
    esac
done

# ---- discovery: pure `find`, needs no lock, no bootstrap -------------------
# Moved ahead of the lock/bootstrap machinery below so --list-only (and, in
# turn, .github/ci-test-scope.sh, which shells out to it for the canonical
# test/crate list) can answer without taking the machine-wide lock or
# requiring rjq/cargo to be ready -- it runs nothing, so nothing it could
# collide with.

# Discover test scripts in a suite, sorted for determinism.
discover() {
    local dir="$1"
    find "$dir" -maxdepth 1 -type f -name 'test-*.sh' -print | sort
}

# Rust crates are separate gate cases so a broken crate cannot hide behind the
# shell-suite result. A contributor without cargo gets an explicit, non-failing
# unconfigured result unless the CI refusal switch is set. Discover every
# workspace member rather than naming one crate: each planning command and
# reusable library is independently testable and must be covered by the gate.
discover_crates() {
    # B203: a bare `sort` inherits the ambient collation, and under
    # LC_ALL=C `src/ai-text-editor-mcp` precedes `src/ai-text-editor/`
    # (dash 0x2D < slash 0x2F) while a UTF-8 shell reverses it — per-crate
    # legs then depend on a machine, not the runner. Pin the collation.
    find "$repo_root/src" -mindepth 2 -maxdepth 2 -type f -name Cargo.toml -print \
        | LC_ALL=C sort \
        | sed "s#^$repo_root/##; s#/Cargo.toml\$##"
}

suites=(
    tests
    planning/tests
    # chat/tests was never discovered here, so every chat assertion — including
    # the rung CI builds cargo for specifically "so its assertions run" (T62) —
    # was dead weight: a green suite proved nothing about the chat skill.
    chat/tests
    # Registered with the suite dir, not after it: an interactive-shell/tests
    # that nothing discovers is the same dead weight chat/tests was.
    interactive-shell/tests
    # Same reasoning: editor-gate-plugin/tests covers the pattern-matching and
    # token mint/consume logic its Bash hard gate and Edit/Write soft
    # reminder both depend on.
    editor-gate-plugin/tests
    # Same reasoning: tui-hint-plugin/tests covers the profile-matching logic
    # its Claude Code hook and opencode plugin both depend on.
    tui-hint-plugin/tests
    # Same reasoning: agent-identity-plugin/tests covers the context-building
    # logic its SubagentStart hook depends on.
    agent-identity-plugin/tests
    # .github/tests covers ci-scope.sh and ci-subjects.sh, which decide how much
    # of the workspace CI compiles, and registers-guard.sh, which decides
    # whether a registers push may reach master WITHOUT review. Undiscovered
    # they would be the same dead weight as chat/tests was: the scripts that can
    # silently narrow every run or widen who writes master, with nothing
    # asserting they only do so on grounds.
    .github/tests
    benchmark/planning/tests
)

tests=()
for suite in "${suites[@]}"; do
    while IFS= read -r t; do
        tests+=("$t")
    done < <(discover "$repo_root/$suite")
done

# The one list of "what counts as a test" -- shell test paths (repo-relative)
# then crate directories, in that fixed order. --list-only prints it verbatim;
# --select-file and --shard both filter it before anything runs.
work_items=()
for t in ${tests[@]+"${tests[@]}"}; do
    work_items+=("${t#"$repo_root"/}")
done
while IFS= read -r crate; do
    [ -n "$crate" ] && work_items+=("$crate")
done < <(discover_crates)

# --select-file: keep only items named in FILE. Unlisted items are dropped
# from work_items entirely, before --shard ever sees them, so a shard index is
# always computed over the same reduced list a human reading FILE would expect
# -- not over the full discovery set with some items later skipped silently.
if [ -n "$select_file" ]; then
    [ -r "$select_file" ] || { printf '%s: cannot read --select-file %s\n' "${0##*/}" "$select_file" >&2; exit 64; }
    kept=()
    for item in "${work_items[@]}"; do
        if grep -Fqx "$item" "$select_file"; then
            kept+=("$item")
        fi
    done
    work_items=(${kept[@]+"${kept[@]}"})
fi

# --shard: deterministic position in the (already select-file'd) sorted list.
# tests[] and discover_crates are each independently sorted, and the two are
# concatenated in a fixed order, so work_items' order is itself deterministic
# -- position i is the same item on every run over the same inputs.
if [ -n "$shard_total" ]; then
    sharded=()
    i=0
    for item in ${work_items[@]+"${work_items[@]}"}; do
        [ $((i % shard_total)) -eq "$shard_index" ] && sharded+=("$item")
        i=$((i + 1))
    done
    work_items=(${sharded[@]+"${sharded[@]}"})
fi

# --list-only prints the FINAL list -- after any --select-file/--shard
# filtering, so it is also how a shard's exact contents are inspected or
# reproduced without actually running anything.
if [ "$list_only" = true ]; then
    printf '%s\n' ${work_items[@]+"${work_items[@]}"}
    exit 0
fi

# Rebuild tests[] (absolute paths, as the rest of the script expects) and the
# crate list from whatever --select-file/--shard left in work_items.
tests=()
selected_crates=()
for item in ${work_items[@]+"${work_items[@]}"}; do
    case "$item" in
        src/*) selected_crates+=("$item") ;;
        *) tests+=("$repo_root/$item") ;;
    esac
done

# ---- one run at a time, machine-wide ---------------------------------------
# Two suite runs on this machine collide. They share the cargo target
# directory, the default chat beacon port (7780), and the short /tmp test roots
# lib-test.sh explains cannot nest under a per-run scratch; a second
# verify-both-shells.sh deletes the first's linked worktree outright. The
# symptom is missing-file failures across most of the suite, or a handful of
# chat failures that vanish on a clean re-run, which reads as a regression and
# is not one.
#
# The path is fixed under /tmp rather than $TMPDIR: a mutex only works if both
# runs agree on where it lives, TMPDIR varies per user and per session, and
# this script exports TMPDIR to its own scratch root a few lines below.
#
# Created with noclobber, so the create IS the test — two runs starting in the
# same instant cannot both win it.
lock_file="/tmp/ai-skills-run-tests.lock"
lock_held=false
lock_marker="ai-skills-run-tests"

# The command line of a live pid, or nothing. Identity matters as much as
# liveness: a pid recorded by a run that was killed can be reassigned to an
# unrelated process, and checking only that "the pid exists" would then block
# every future run forever with no way to tell why.
lock_holder_command() {
    ps -p "$1" -o args= 2>/dev/null || ps -p "$1" -o command= 2>/dev/null || true
}

lock_holder_is_live() { # <pid> -> 0 when that pid is really a suite run
    local pid="$1" command
    case "$pid" in ''|*[!0-9]*) return 1 ;; esac
    command="$(lock_holder_command "$pid")"
    [ -n "$command" ] || return 1
    case "$command" in *run-tests.sh*) return 0 ;; *) return 1 ;; esac
}

take_lock() { # 0 on success
    ( set -o noclobber
      printf '%s\n%s\n%s\n%s\n' \
          "$$" "$lock_marker" "$repo_root" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
          > "$lock_file" ) 2>/dev/null
}

release_lock() {
    [ "$lock_held" = true ] || return 0
    # Only ever remove our own: a run that refused to start must not delete the
    # lock belonging to the run that is legitimately holding it.
    if [ "$(sed -n '1p' "$lock_file" 2>/dev/null)" = "$$" ]; then
        rm -f -- "$lock_file"
    fi
}

if [ "${AI_SKILLS_ALLOW_CONCURRENT:-0}" = 1 ]; then
    printf 'run-tests: AI_SKILLS_ALLOW_CONCURRENT=1; the single-run lock is bypassed\n' >&2
elif take_lock; then
    lock_held=true
else
    held_pid="$(sed -n '1p' "$lock_file" 2>/dev/null)"
    if lock_holder_is_live "$held_pid"; then
        printf 'run-tests: another suite run is already going (pid %s)\n' "$held_pid" >&2
        printf '  started: %s\n' "$(sed -n '4p' "$lock_file" 2>/dev/null)" >&2
        printf '  in:      %s\n' "$(sed -n '3p' "$lock_file" 2>/dev/null)" >&2
        printf '  command: %s\n' "$(lock_holder_command "$held_pid")" >&2
        printf '  Two runs on one machine collide over the cargo target dir, the\n' >&2
        printf '  chat beacon port and the /tmp test roots. Wait for it, or set\n' >&2
        printf '  AI_SKILLS_ALLOW_CONCURRENT=1 to run anyway and accept the noise.\n' >&2
        exit 75
    fi
    # Stale: the recorded pid is gone, or belongs to something that is not a
    # suite run. Reuse it. The retry is what settles a race between two runs
    # that both found it stale -- whoever creates it first owns it, and the
    # other falls through to the refusal above on its next pass.
    printf 'run-tests: reusing a stale lock left by pid %s\n' "${held_pid:-unknown}" >&2
    rm -f -- "$lock_file"
    if take_lock; then
        lock_held=true
    else
        held_pid="$(sed -n '1p' "$lock_file" 2>/dev/null)"
        printf 'run-tests: another run took the lock first (pid %s); not starting\n' \
            "${held_pid:-unknown}" >&2
        exit 75
    fi
fi

# Scope everything this run creates under one scratch root and clean it up on
# exit so no temp files survive a pass, fail, or interrupted run. The planning
# skill writes scratch under ${TMPDIR}/planning-agent, the benchmark runner
# places capsules under PLANNING_AGENT_TMPDIR, and archives are staged under
# benchmark/results/<agent>/.staging/ — all three are covered below.
run_scratch="$(mktemp -d "${TMPDIR:-/tmp}/ai-skills-tests.XXXXXX")"
test_output="$run_scratch/test-output.txt"
AI_SKILLS_TEST_RUN_ID="run-tests.$$.$(date -u +%s)"
export AI_SKILLS_TEST_RUN_ID
export TMPDIR="$run_scratch"
export PLANNING_AGENT_TMPDIR="$run_scratch/planning-agent"
cleanup_marked_test_roots() {
    local scan candidate marker marker_value
    for scan in /tmp "${TMPDIR:-}"; do
        [ -n "$scan" ] || continue
        [ -d "$scan" ] || continue
        while IFS= read -r candidate; do
            [ -n "$candidate" ] || continue
            marker="$candidate/.ai-skills-test-run-id"
            [ -f "$marker" ] || continue
            marker_value="$(sed -n '1p' "$marker")"
            [ "$marker_value" = "$AI_SKILLS_TEST_RUN_ID" ] || continue
            rm -rf -- "$candidate"
        done < <(find -H "$scan" -maxdepth 1 -type d -name 't.?????' -print 2>/dev/null)
    done
}
cleanup() {
    release_lock
    cleanup_marked_test_roots
    rm -rf -- "$run_scratch"
    # Each test takes a short root directly under /tmp -- lib-test.sh explains
    # why it cannot nest under this scratch. The run-id marker lets this cleanup
    # remove only roots from this suite run, even if a test overwrote lib-test's
    # EXIT trap after sourcing it.
    rm -rf -- "$repo_root/benchmark/results"/*/.staging
}
trap cleanup EXIT

# Generated artifacts are never committed (MAINTAINER.md section 2.16), so a
# clean checkout has none of them. Build-if-missing here; staleness detection
# stays with the tests, so the bootstrap cannot mask drift. A missing rjq is
# fatal with the fix named: the register tests cannot run without it.
# B156: refuse the WHOLE suite fast, with one clear message, rather than
# every test failing individually with the same complaint. lib-test.sh's
# t_begin carries the same check (repo_root-independent, since not every test
# sources this script's variables) for a test run standalone outside the
# suite runner; see setup-dev-env.sh's header for what writes the markers.
refuse_if_dev_env_dirty() {
    local started="$repo_root/.setup-dev-env.started"
    local finished="$repo_root/.setup-dev-env.finished"
    local started_token finished_token
    [ -f "$started" ] || return 0
    started_token="$(cat "$started" 2>/dev/null)"
    finished_token=""
    [ -f "$finished" ] && finished_token="$(cat "$finished" 2>/dev/null)"
    [ -n "$started_token" ] && [ "$started_token" = "$finished_token" ] && return 0
    printf '%s: a setup-dev-env.sh run started and never finished (or finished a\n' "${0##*/}" >&2
    printf '  different run) -- the build tree is in an unknown, possibly partial\n' >&2
    printf '  state, which is the leading suspect behind this suite failing then\n' >&2
    printf '  passing on an identical tree (B156). Finish it, then re-run:\n' >&2
    printf '    ./setup-dev-env.sh\n' >&2
    exit 70
}
refuse_if_dev_env_dirty

bootstrap_generated() {
    local lib missing=0 dir
    for lib in plan-core-lib.sh plan-crypt-lib.sh plan-document-lib.sh plan-progress-lib.sh plan-table-lib.sh; do
        [ -f "$repo_root/planning/scripts/$lib" ] || missing=1
    done
    if [ "$missing" -eq 1 ]; then
        "$repo_root/planning/scripts/build-plan-libs.sh" >&2
    fi
    if [ ! -f "$repo_root/planning/REVIEWER.md" ]; then
        "$repo_root/planning/scripts/generate-reviewer.sh" >&2
    fi
    if ! command -v rjq >/dev/null 2>&1; then
        dir=""
        if dir="$("$repo_root/bootstrap.sh" rjq --path-only 2>/dev/null)" && [ -n "$dir" ]; then
            PATH="$dir:$PATH"
            export PATH
        elif ! command -v rjq >/dev/null 2>&1; then
            printf '%s: rjq is still missing; the register tests cannot run.\n' "${0##*/}" >&2
            exit 69
        fi
    fi
}
bootstrap_generated

# Tests that require PLANNING_CONTEXT_CACHE (a developer-only legacy context
# cache). They are documented to fail closed when the fixture is absent; the
# runner prefers to run them when the fixture is configured and otherwise
# reports them as UNCONFIGURED rather than as a silent skip or a hard failure.
context_gated=(
    planning/tests/test-plan-context.sh
    planning/tests/test-plan-context-deferred-boundary.sh
)

# $1 is an absolute test path; context_gated lists repo-relative paths.
is_context_gated() {
    local t="${1#"$repo_root"/}" entry
    for entry in "${context_gated[@]}"; do
        [ "$entry" = "$t" ] && return 0
    done
    return 1
}

total=0
passed=0
failed=0
skipped=0
unconfigured=0
declare -a failed_names=()
declare -a skipped_names=()
declare -a unconfigured_names=()

# How one test's outcome is reported and counted. Split out of run_one to keep
# both inside CODE-STYLE.md's 40-line cap, and because "how a result is shown"
# is a different concern from "how the test is launched".
report_one() { # <label> <exit-code>
    local label="$1" code="$2"
    if [ "$code" -eq 0 ]; then
        # t_skip exits 0, same as a real pass (B268: a missing precondition is
        # not a failure) -- only the trailing "<test>: SKIP" line in its own
        # output tells the two apart.
        if grep -q ': SKIP$' "$test_output" 2>/dev/null; then
            skipped=$((skipped + 1))
            skipped_names+=("$label")
            printf '  %-52s SKIP\n' "$label"
            sed 's/^/      /' "$test_output"
            return 0
        fi
        passed=$((passed + 1))
        printf '  %-52s PASS\n' "$label"
        [ "$verbose" = true ] && sed 's/^/      /' "$test_output"
        return 0
    fi
    failed=$((failed + 1))
    failed_names+=("$label")
    # 124 is timeout(1)'s own code for "the deadline passed", and it is reported
    # as its own outcome. A test that never finished and a test that answered
    # wrongly need different next steps, and a summary calling them both FAIL
    # sends the reader looking for an assertion that does not exist.
    if [ "$code" -eq 124 ]; then
        printf '  %-52s TIMEOUT (%ss)\n' "$label" "$test_timeout_seconds"
    else
        printf '  %-52s FAIL (exit %s)\n' "$label" "$code"
    fi
    # Always the whole output, --verbose or not. On a timeout this is what the
    # test had printed before it stopped, which is what says where.
    sed 's/^/      /' "$test_output"
}

run_one() {
    local t="$1" label mem cpu code=0
    label="$(sed 's#^.*/tests/##; s#\.sh$##' <<<"$t")"
    # benchmark tests spin up worker/reviewer-like processes; give them headroom.
    case "$t" in
        "$repo_root"/benchmark/*) mem=6G; cpu=400 ;;
        *) mem=2G; cpu=400 ;;
    esac

    if is_context_gated "$t" && [ -z "${PLANNING_CONTEXT_CACHE:-}" ]; then
        unconfigured=$((unconfigured + 1))
        unconfigured_names+=("$label")
        printf '  %-52s UNCONFIGURED (PLANNING_CONTEXT_CACHE)\n' "$label"
        return
    fi

    total=$((total + 1))
    # Built as a list because both layers are optional: the wrapper is dropped
    # on CI and the timeout where timeout(1) is missing. The array is never
    # empty, so `set -u` needs no guard here.
    local -a argv=()
    [ -n "$timeout_cmd" ] && argv+=("$timeout_cmd" "$test_timeout_seconds")
    [ -n "$wrapper" ] && argv+=("$wrapper" "$mem" "$cpu" --)
    argv+=("$BASH" "$t")

    "${argv[@]}" >"$test_output" 2>&1 || code=$?
    report_one "$label" "$code"
    rm -f "$test_output"
}

start="$(date -u +%s)"
echo "Testing $(basename "$repo_root") — $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo "Runner order: sorted test files under planning/tests then benchmark/planning/tests"
echo

for t in ${tests[@]+"${tests[@]}"}; do
    run_one "$t"
done

run_cargo_one() {
    local crate="$1"
    local label="cargo-${crate##*/}"
    if ! command -v cargo >/dev/null 2>&1; then
        if [ "${REFUSE_UNCONFIGURED_CARGO:-0}" = 1 ]; then
            failed=$((failed + 1))
            failed_names+=("$label")
            printf '  %-52s FAIL (cargo unavailable; REFUSE_UNCONFIGURED_CARGO=1)\n' "$label"
        else
            unconfigured=$((unconfigured + 1))
            unconfigured_names+=("$label")
            printf '  %-52s UNCONFIGURED (cargo)\n' "$label"
        fi
        return
    fi
    total=$((total + 1))
    # Same optional layers as run_one, and for the same reason: with the
    # wrapper dropped on CI an unguarded "$wrapper" expands to an empty word,
    # which the shell tries to execute and reports as a command not found.
    local -a argv=()
    [ -n "$timeout_cmd" ] && argv+=("$timeout_cmd" "$test_timeout_seconds")
    [ -n "$wrapper" ] && argv+=("$wrapper" 2G 400 --)
    argv+=(cargo test --manifest-path "$repo_root/$crate/Cargo.toml")

    local code=0
    "${argv[@]}" >"$test_output" 2>&1 || code=$?
    report_one "$label" "$code"
}

for crate in ${selected_crates[@]+"${selected_crates[@]}"}; do
    run_cargo_one "$crate"
done

elapsed="$(( $(date -u +%s) - start ))"
echo
echo "──────────────────────────────────────────────"
printf 'Total ran: %d   Passed: %d   Failed: %d   Skipped: %d   Unconfigured: %d\n' \
    "$total" "$passed" "$failed" "$skipped" "$unconfigured"
printf 'Elapsed: %ds\n' "$elapsed"
# bash 3.2 treats "${arr[@]}" of an empty array as unbound under `set -u`.
if [ -n "${failed_names[*]+set}" ] && [ "${#failed_names[@]}" -gt 0 ]; then
    printf 'Failed: %s\n' "${failed_names[*]-}"
fi
if [ -n "${skipped_names[*]+set}" ] && [ "${#skipped_names[@]}" -gt 0 ]; then
    printf 'Skipped: %s\n' "${skipped_names[*]-}"
fi
if [ -n "${unconfigured_names[*]+set}" ] && [ "${#unconfigured_names[@]}" -gt 0 ]; then
    printf 'Unconfigured (set PLANNING_CONTEXT_CACHE to run): %s\n' "${unconfigured_names[*]-}"
fi
echo "──────────────────────────────────────────────"

[ "$failed" -eq 0 ]
