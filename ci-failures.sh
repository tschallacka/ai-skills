#!/usr/bin/env bash
# MODE: DEV
# ci-failures - what actually failed in a CI run, from a run id, a PR number or
# a branch.
#
# Usage:
#   ci-failures.sh                  # the newest run for the current branch
#   ci-failures.sh 33894205595      # a run id
#   ci-failures.sh 47               # a PR number
#   ci-failures.sh pr/47            # a PR number, unambiguously
#   ci-failures.sh fix/some-branch  # the newest run for a branch
#   ci-failures.sh <target> --raw   # keep the whole log of each failing job
#   ci-failures.sh <target> --all   # every job, not only the failing ones
#
# It prints, per failing job, the lines that identify the failure: the suite's
# own `Failed:` summary, cargo's `test result:`, every panic with the lines
# after it, `##[error]` annotations, and this repository's own `FAIL`/
# `portability:` findings. `--raw DIR` additionally writes each job's full,
# de-escaped log to DIR so a long screen dump can be read whole.
#
# WHY THIS EXISTS. Reading a failing run by hand is six steps -- list the jobs,
# find the failing ids, fetch each log through the API, allow the escape
# sequences, strip the CR and the ANSI, then search for the interesting lines --
# and it was done eight times in one session before anyone wrote it down.
#
# Three things about that dance are not guessable, and are the reason this is a
# script rather than a note:
#
#   1. `gh run view` REFUSES while a run is in progress. The per-job logs API
#      does not, so a failing leg can be read without waiting for its siblings
#      or cancelling anything.
#   2. `gh api .../logs` refuses the body outright unless
#      --allow-escape-sequences is passed, because CI logs carry colour codes.
#   3. Those codes then have to be stripped, and `\x1b` is a GNU sed extension.
#      The ESC has to be a real character in the pattern for BSD sed, which is
#      the userland this repository targets.
set -euo pipefail
export LC_ALL=C

repo_slug="${CI_FAILURES_REPO:-tschallacka/ai-skills}"
target="${1:-}"
raw_dir=''
want_all=false

case "$target" in
    -h|--help)
        # The usage block above, minus the shebang and the marker.
        awk 'NR <= 2 { next }
             /^#/ { sub(/^#[[:space:]]?/, ""); print; next }
             { exit }' "$0"
        exit 0
        ;;
esac

shift || true
while [ "$#" -gt 0 ]; do
    case "$1" in
        --raw)
            [ "$#" -ge 2 ] || { printf 'ci-failures: --raw needs a directory\n' >&2; exit 64; }
            raw_dir="$2"
            shift 2
            ;;
        --raw=*) raw_dir="${1#--raw=}"; shift ;;
        --all) want_all=true; shift ;;
        *) printf 'ci-failures: unknown option: %s\n' "$1" >&2; exit 64 ;;
    esac
done

command -v gh >/dev/null 2>&1 || { printf 'ci-failures: gh is required\n' >&2; exit 69; }

# ESC as a literal byte. `\x1b` is a GNU sed extension and this repository
# targets a BSD userland too, so the pattern carries the character itself.
esc="$(printf '\033')"

# Resolve the target to a run id.
#
# The order matters. A bare number is ambiguous -- run ids and PR numbers are
# both integers -- and the disambiguation is by MAGNITUDE, which is a heuristic
# and therefore stated out loud rather than hidden: a GitHub Actions run id is a
# 10+ digit snowflake, a PR number in this repository is two or three digits.
# `pr/47` says which is meant when that guess is not good enough.
resolve_run() {
    local want="$1" branch head_branch
    case "$want" in
        pr/*)
            head_branch="$(gh pr view "${want#pr/}" --repo "$repo_slug" \
                --json headRefName --jq .headRefName)" \
                || { printf 'ci-failures: no PR %s\n' "${want#pr/}" >&2; exit 66; }
            latest_run_for "$head_branch"
            return
            ;;
        '')
            branch="$(git symbolic-ref --short -q HEAD || true)"
            [ -n "$branch" ] || { printf 'ci-failures: detached HEAD; name a run, PR or branch\n' >&2; exit 64; }
            latest_run_for "$branch"
            return
            ;;
    esac
    case "$want" in
        *[!0-9]*)
            latest_run_for "$want"
            return
            ;;
    esac
    if [ "${#want}" -ge 9 ]; then
        printf '%s\n' "$want"
        return
    fi
    head_branch="$(gh pr view "$want" --repo "$repo_slug" \
        --json headRefName --jq .headRefName)" \
        || { printf 'ci-failures: %s is neither a run id nor a PR\n' "$want" >&2; exit 66; }
    latest_run_for "$head_branch"
}

# The newest run for a branch, ignoring the artifact-render workflow: it is
# almost always green and never the reason a check suite is red.
latest_run_for() {
    local branch="$1" id
    id="$(gh run list --repo "$repo_slug" --branch "$branch" --limit 20 \
        --json databaseId,name --jq '[.[] | select(.name != "render-artifacts")][0].databaseId')"
    [ -n "$id" ] && [ "$id" != null ] \
        || { printf 'ci-failures: no runs for branch %s\n' "$branch" >&2; exit 66; }
    printf '%s\n' "$id"
}

run_id="$(resolve_run "$target")"

status="$(gh run view "$run_id" --repo "$repo_slug" --json status --jq .status)"
conclusion="$(gh run view "$run_id" --repo "$repo_slug" --json conclusion --jq '.conclusion // "pending"')"
printf 'run %s  %s/%s  https://github.com/%s/actions/runs/%s\n' \
    "$run_id" "$status" "$conclusion" "$repo_slug" "$run_id"

# A job whose conclusion is empty is still running; `select(.conclusion ==
# "failure")` therefore reports only settled failures, which is what "what
# failed" means. --all keeps every job so a run can be surveyed.
if [ "$want_all" = true ]; then
    jobs="$(gh run view "$run_id" --repo "$repo_slug" --json jobs \
        --jq '.jobs[] | (.databaseId|tostring) + "\t" + (.conclusion // "running") + "\t" + .name')"
else
    jobs="$(gh run view "$run_id" --repo "$repo_slug" --json jobs \
        --jq '.jobs[] | select(.conclusion == "failure") | (.databaseId|tostring) + "\tfailure\t" + .name')"
fi

if [ -z "$jobs" ]; then
    if [ "$conclusion" = failure ]; then
        printf '\nno job reports failure yet, though the run does: it may still be settling,\n'
        printf 'or the failure is at the workflow level (a cancelled or skipped required job).\n'
        printf 'Re-run with --all to see every job.\n'
        exit 0
    fi
    printf '\nno failing jobs.\n'
    exit 0
fi

[ -z "$raw_dir" ] || mkdir -p "$raw_dir"

# The patterns worth printing, learned from the failures this repository
# actually produces. `panicked at` pulls the four lines after it, because a
# Rust panic's message and this repo's added diagnostics -- the last screen
# rows, the wrapper's stderr, the last connect error -- are on the lines below
# the location.
extract() {
    awk -v esc="$esc" '
        { gsub(esc "\\[[0-9;]*[a-zA-Z]", ""); sub(/\r$/, "") }
        # Drop the leading ISO timestamp the API prefixes to every line.
        { line = $0; sub(/^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9:.]+Z[[:space:]]?/, "", line) }
        # A failing test row is followed by its own findings, indented deeper
        # than the row. Continuing on indentation rather than on a line count
        # is what keeps the next test PASS row out of the extract: a fixed
        # window pulled in three of them.
        detail && line ~ /^[[:space:]][[:space:]][[:space:]][[:space:]]/ { print "    " line; next }
        detail { detail = 0 }
        after > 0 { print "    " line; after--; next }
        line ~ /panicked at/ { print "    " line; after = 4; next }
        line ~ /^[[:space:]]*Failed:/ { print "    " line; next }
        line ~ /Total ran:/ { print "    " line; next }
        line ~ /test result: FAILED/ { print "    " line; next }
        line ~ /##\[error\]/ { print "    " line; next }
        # `run-tests.sh` prints "  <name>    FAIL (exit 1)" and then the test
        # own findings, indented, on the lines below. The space before the
        # paren is why a `FAIL[:(]` pattern missed the whole class -- caught by
        # this script failing to explain a ratchet failure it was pointed at.
        line ~ /FAIL[[:space:]]*[:(]/ { print "    " line; detail = 1; next }
        line ~ /^[[:space:]]*(FAIL|portability):/ { print "    " line; next }
        line ~ /^error(\[|:)/ { print "    " line; next }
        line ~ /timed out waiting/ { print "    " line; after = 2; next }
    '
}

printf '%s\n' "$jobs" | while IFS="$(printf '\t')" read -r job_id job_conclusion job_name; do
    [ -n "$job_id" ] || continue
    printf '\n== %s  [%s]  job %s\n' "$job_name" "$job_conclusion" "$job_id"
    # --allow-escape-sequences is required: gh refuses a body carrying terminal
    # colour codes, and every CI log carries them. Without it this prints
    # nothing at all and the run looks empty.
    log="$(gh api --allow-escape-sequences \
        "repos/$repo_slug/actions/jobs/$job_id/logs" 2>/dev/null || true)"
    if [ -z "$log" ]; then
        printf '    (no log; a job that never started has none)\n'
        continue
    fi
    if [ -n "$raw_dir" ]; then
        printf '%s\n' "$log" | sed -e "s/${esc}\[[0-9;]*[a-zA-Z]//g" -e 's/\r$//' \
            > "$raw_dir/$job_id.log"
        printf '    raw: %s/%s.log\n' "$raw_dir" "$job_id"
    fi
    found="$(printf '%s\n' "$log" | extract)"
    if [ -n "$found" ]; then
        printf '%s\n' "$found"
    else
        printf '    (nothing matched the failure patterns; read the raw log)\n'
    fi
done
