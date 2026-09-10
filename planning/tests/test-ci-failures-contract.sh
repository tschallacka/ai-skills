#!/usr/bin/env bash
# MODE: DEV
# COVERS: ci-failures/scripts/ci-failures.sh
# test-ci-failures-contract — ci-failures.sh's own logic: forge detection, the
# gh and glab target-resolution paths, and the shared extractor.
#
# gh is exercised against this repository's real GitHub Actions runs by hand
# (see AGENTS.md); there is no live GitLab remote to do the same for glab, so
# every glab call here is a stub that logs its argv and returns canned JSON --
# this file is the only place the glab path is exercised at all. Treat a first
# real run against a GitLab project as the still-missing verification step for
# that path, not as settled by this file passing.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
script="$repo_dir/ci-failures/scripts/ci-failures.sh"
work="$(mktemp -d "${TMPDIR:-/tmp}/ci-failures-contract.XXXXXX")"
trap 'rm -rf "$work"' EXIT

note_fail() { printf 'ci-failures: %s\n' "$1" >&2; t_record "$1"; }

# A real rjq is required (the script refuses to run without one); find the
# one this checkout built rather than assuming it is on PATH.
rjq_bin=""
for candidate in "$repo_dir/target/release/rjq" "$repo_dir/bin"/*/rjq; do
    [ -x "$candidate" ] && { rjq_bin="$candidate"; break; }
done
[ -n "$rjq_bin" ] || t_skip "no built rjq found (run ./setup-dev-env.sh)"

stub_bin="$work/bin"
mkdir -p "$stub_bin"
ln -s "$rjq_bin" "$stub_bin/rjq"
logs_dir="$work/logs"
mkdir -p "$logs_dir"

# A fake git repo, so `git remote get-url origin` and `git symbolic-ref` are
# real, not stubbed -- the script's own remote parsing is what is under test.
repo_work="$work/repo"
mkdir -p "$repo_work"
git -C "$repo_work" init -q -b some-branch
git_remote() { git -C "$repo_work" remote remove origin 2>/dev/null || true; git -C "$repo_work" remote add origin "$1"; }

cat >"$stub_bin/gh" <<'STUB'
#!/usr/bin/env bash
argv="$*"
printf 'gh %s\n' "$argv" >>"$STUB_LOG"
case "$argv" in
    "repo view"*)
        [ "${STUB_GH_AVAILABLE:-1}" = 1 ] && exit 0 || exit 1
        ;;
    *"pr view "*"--json headRefName"*)
        printf '{"headRefName":"%s"}\n' "$STUB_HEAD_BRANCH"
        ;;
    *"run list "*"--json databaseId,name"*)
        printf '%s\n' "$STUB_RUN_LIST_JSON"
        ;;
    *"run view "*"--json status"*)
        printf '{"status":"%s"}\n' "$STUB_RUN_STATUS"
        ;;
    *"run view "*"--json conclusion"*)
        printf '{"conclusion":"%s"}\n' "$STUB_RUN_CONCLUSION"
        ;;
    *"run view "*"--json jobs"*)
        printf '{"jobs":%s}\n' "$STUB_JOBS_JSON"
        ;;
    *"api --allow-escape-sequences "*"/logs")
        job_id="${argv##*/actions/jobs/}"
        job_id="${job_id%/logs}"
        cat "$STUB_LOGS_DIR/$job_id.log" 2>/dev/null || true
        ;;
    *) exit 1 ;;
esac
STUB
chmod +x "$stub_bin/gh"

cat >"$stub_bin/glab" <<'STUB'
#!/usr/bin/env bash
argv="$*"
printf 'glab %s\n' "$argv" >>"$STUB_LOG"
case "$argv" in
    "repo view"*)
        [ "${STUB_GLAB_AVAILABLE:-1}" = 1 ] && exit 0 || exit 1
        ;;
    *"/merge_requests/"*)
        printf '{"source_branch":"%s"}\n' "$STUB_MR_BRANCH"
        ;;
    *"/pipelines?ref="*)
        printf '%s\n' "$STUB_PIPELINE_LIST_JSON"
        ;;
    *"/pipelines/"*"/jobs?"*)
        printf '%s\n' "$STUB_JOBS_JSON"
        ;;
    *"/jobs/"*"/trace")
        job_id="${argv##*/jobs/}"
        job_id="${job_id%/trace}"
        cat "$STUB_LOGS_DIR/$job_id.log" 2>/dev/null || true
        ;;
    *"/pipelines/"*)
        printf '{"status":"%s"}\n' "$STUB_PIPELINE_STATUS"
        ;;
    *) exit 1 ;;
esac
STUB
chmod +x "$stub_bin/glab"

run_script() {
    : >"$work/call.log"
    set +e
    ( cd "$repo_work" \
        && STUB_LOG="$work/call.log" STUB_LOGS_DIR="$logs_dir" \
           PATH="$stub_bin:$PATH" "$BASH" "$script" "$@" )  \
        >"$work/out" 2>"$work/err"
    RUN_RC=$?
    set -e
    RUN_OUT="$(cat "$work/out")"
    RUN_ERR="$(cat "$work/err")"
    RUN_CALLS="$(cat "$work/call.log" 2>/dev/null || true)"
}

# ─────────────────────────────────────────────────────────────────────────────
# Forge detection
# ─────────────────────────────────────────────────────────────────────────────
export STUB_GH_AVAILABLE=1 STUB_GLAB_AVAILABLE=1
export STUB_HEAD_BRANCH=some-branch STUB_RUN_STATUS=completed STUB_RUN_CONCLUSION=success
export STUB_RUN_LIST_JSON='[{"databaseId":123456789,"name":"build"}]'
export STUB_JOBS_JSON='[]'
export STUB_MR_BRANCH=some-branch STUB_PIPELINE_STATUS=success
export STUB_PIPELINE_LIST_JSON='[{"id":987654321}]'

git_remote https://github.com/tschallacka/ai-skills.git
run_script
case "$RUN_OUT" in
    'forge: gh'*) ;;
    *) note_fail "a github.com remote did not choose gh: $RUN_OUT" ;;
esac

git_remote git@gitlab.com:tschallacka/ai-skills.git
run_script
case "$RUN_OUT" in
    'forge: glab'*) ;;
    *) note_fail "a gitlab.com remote did not choose glab: $RUN_OUT" ;;
esac

# An override outranks the remote outright.
git_remote https://github.com/tschallacka/ai-skills.git
( cd "$repo_work" && CI_FAILURES_FORGE=glab STUB_LOG="$work/call.log" \
    STUB_LOGS_DIR="$logs_dir" PATH="$stub_bin:$PATH" "$BASH" "$script" \
    >"$work/out" 2>"$work/err" )
RUN_OUT="$(cat "$work/out")"
case "$RUN_OUT" in
    'forge: glab'*) ;;
    *) note_fail "CI_FAILURES_FORGE=glab did not override a github.com remote: $RUN_OUT" ;;
esac

# A self-hosted remote (neither host substring) falls back to whichever CLI
# answers repo view.
git_remote https://ci.example.internal/team/project.git
STUB_GH_AVAILABLE=0 STUB_GLAB_AVAILABLE=1 run_script
case "$RUN_OUT" in
    'forge: glab'*) ;;
    *) note_fail "a self-hosted remote with only glab authenticated did not fall back to glab: $RUN_OUT" ;;
esac

STUB_GH_AVAILABLE=0 STUB_GLAB_AVAILABLE=0 run_script
[ "$RUN_RC" -eq 66 ] || note_fail "a self-hosted remote with neither CLI authenticated exited $RUN_RC, expected 66"
case "$RUN_ERR" in
    *'could not tell which forge'*) ;;
    *) note_fail "the refusal did not explain itself: $RUN_ERR" ;;
esac
STUB_GH_AVAILABLE=1 STUB_GLAB_AVAILABLE=1

# An unrecognised override is refused by name, not silently ignored.
CI_FAILURES_FORGE=svn run_script
[ "$RUN_RC" -eq 64 ] || note_fail "CI_FAILURES_FORGE=svn exited $RUN_RC, expected 64"
case "$RUN_ERR" in
    *'must be gh or glab, not svn'*) ;;
    *) note_fail "the bad override was not named back: $RUN_ERR" ;;
esac

# ─────────────────────────────────────────────────────────────────────────────
# gh path: target resolution
# ─────────────────────────────────────────────────────────────────────────────
git_remote https://github.com/tschallacka/ai-skills.git

# A 9+ digit number is a run id directly -- no PR lookup.
run_script 3389420559
case "$RUN_CALLS" in
    *'pr view'*) note_fail "a 10-digit target triggered a PR lookup: $RUN_CALLS" ;;
esac
case "$RUN_CALLS" in
    *'run view 3389420559'*) ;;
    *) note_fail "a 10-digit target was not read as a run id: $RUN_CALLS" ;;
esac

# A short number is read as a PR number, resolved to a branch, then a run.
run_script 47
case "$RUN_CALLS" in
    *'pr view 47 '*) ;;
    *) note_fail "a short number did not resolve as a PR: $RUN_CALLS" ;;
esac

# pr/N is unambiguous regardless of magnitude.
run_script pr/47
case "$RUN_CALLS" in
    *'pr view 47 '*) ;;
    *) note_fail "pr/47 did not resolve as a PR: $RUN_CALLS" ;;
esac

# Empty target reads the current branch.
run_script
case "$RUN_CALLS" in
    *'run list'*'--branch some-branch'*) ;;
    *) note_fail "an empty target did not use the current branch: $RUN_CALLS" ;;
esac

# CI_FAILURES_REPO overrides the hardcoded default slug.
CI_FAILURES_REPO=other/repo run_script pr/47
case "$RUN_CALLS" in
    *'--repo other/repo'*) ;;
    *) note_fail "CI_FAILURES_REPO was not honoured on the gh path: $RUN_CALLS" ;;
esac

# --all lists every job; the default lists only failures.
export STUB_JOBS_JSON='[{"databaseId":1,"conclusion":"success","name":"a"},{"databaseId":2,"conclusion":"failure","name":"b"}]'
run_script pr/47
case "$RUN_OUT" in
    *'== a '*) note_fail "the default gh listing included a passing job: $RUN_OUT" ;;
esac
case "$RUN_OUT" in
    *'== b '*) ;;
    *) note_fail "the default gh listing dropped the failing job: $RUN_OUT" ;;
esac
run_script pr/47 --all
case "$RUN_OUT" in
    *'== a '*) ;;
    *) note_fail "--all dropped a passing job on gh: $RUN_OUT" ;;
esac
export STUB_JOBS_JSON='[]'

# ─────────────────────────────────────────────────────────────────────────────
# glab path: target resolution
# ─────────────────────────────────────────────────────────────────────────────
git_remote git@gitlab.com:tschallacka/some-group/ai-skills.git

# The project path is percent-encoded, subgroup included, from the scp-like form.
run_script pr/47
case "$RUN_CALLS" in
    *'projects/tschallacka%2Fsome-group%2Fai-skills/merge_requests/47'*) ;;
    *) note_fail "the scp-like remote did not encode the nested project path: $RUN_CALLS" ;;
esac

git_remote https://gitlab.com/tschallacka/ai-skills.git
run_script pr/47
case "$RUN_CALLS" in
    *'projects/tschallacka%2Fai-skills/merge_requests/47'*) ;;
    *) note_fail "the https remote did not encode the project path: $RUN_CALLS" ;;
esac

# pr/N addresses a merge request, then its source branch's latest pipeline.
case "$RUN_CALLS" in
    *'merge_requests/47'*) ;;
    *) note_fail "pr/47 on glab did not look up a merge request: $RUN_CALLS" ;;
esac
case "$RUN_CALLS" in
    *"pipelines?ref=$STUB_MR_BRANCH"*) ;;
    *) note_fail "pr/47 on glab did not then look up the branch's pipeline: $RUN_CALLS" ;;
esac

# A 9+ digit number is a pipeline id directly -- no merge-request lookup.
run_script 987654321012
case "$RUN_CALLS" in
    *'merge_requests'*) note_fail "a large glab target triggered a merge-request lookup: $RUN_CALLS" ;;
esac
case "$RUN_CALLS" in
    *'pipelines/987654321012'*) ;;
    *) note_fail "a large glab target was not read as a pipeline id: $RUN_CALLS" ;;
esac

# Empty target reads the current branch.
run_script
case "$RUN_CALLS" in
    *"pipelines?ref=some-branch"*) ;;
    *) note_fail "an empty target did not use the current branch on glab: $RUN_CALLS" ;;
esac

# CI_FAILURES_REPO bypasses remote parsing entirely (and is used unencoded,
# same as the gh path's repo_slug: the caller is expected to give the already
# slash-joined form).
CI_FAILURES_REPO='grp/proj' run_script pr/9
case "$RUN_CALLS" in
    *'projects/grp%2Fproj/merge_requests/9'*) ;;
    *) note_fail "CI_FAILURES_REPO was not honoured on the glab path: $RUN_CALLS" ;;
esac

# --all lists every job; the default lists only failures (glab statuses use
# "failed", not gh's "failure").
export STUB_JOBS_JSON='[{"id":1,"status":"success","name":"a"},{"id":2,"status":"failed","name":"b"}]'
run_script pr/47
case "$RUN_OUT" in
    *'== a '*) note_fail "the default glab listing included a passing job: $RUN_OUT" ;;
esac
case "$RUN_OUT" in
    *'== b '*) ;;
    *) note_fail "the default glab listing dropped the failing job: $RUN_OUT" ;;
esac
run_script pr/47 --all
case "$RUN_OUT" in
    *'== a '*) ;;
    *) note_fail "--all dropped a passing job on glab: $RUN_OUT" ;;
esac
export STUB_JOBS_JSON='[]'

# ─────────────────────────────────────────────────────────────────────────────
# The extractor: shared by both forges, so proving it once on the glab path
# (whose log carries no ISO-timestamp prefix, unlike gh's) covers both.
# ─────────────────────────────────────────────────────────────────────────────
export STUB_JOBS_JSON='[{"id":1,"status":"failed","name":"tests"}]'
cat >"$logs_dir/1.log" <<'LOG'
running the suite
thread 'main' panicked at src/lib.rs:42:5:
called `Option::unwrap()` on a `None` value
note: run with RUST_BACKTRACE=1 for a backtrace
  test-something    FAIL (exit 1)
    reason: assertion failed
test result: FAILED. 3 passed; 1 failed
LOG
run_script pr/47
case "$RUN_OUT" in
    *"panicked at src/lib.rs:42:5:"*) ;;
    *) note_fail "the extractor missed a panic in an untimestamped (glab-shaped) log: $RUN_OUT" ;;
esac
case "$RUN_OUT" in
    *'FAIL (exit 1)'*) ;;
    *) note_fail "the extractor missed the space-before-paren FAIL form: $RUN_OUT" ;;
esac
case "$RUN_OUT" in
    *'reason: assertion failed'*) ;;
    *) note_fail "the extractor dropped the indented detail under a FAIL row: $RUN_OUT" ;;
esac
case "$RUN_OUT" in
    *'test result: FAILED'*) ;;
    *) note_fail "the extractor missed test result: FAILED: $RUN_OUT" ;;
esac

# ── --raw writes the de-escaped log, colour codes and all ──────────────────
raw_dir="$work/raw"
printf 'plain\n\033[31mred\033[0m\r\n' >"$logs_dir/1.log"
run_script pr/47 --raw "$raw_dir"
[ -f "$raw_dir/1.log" ] || note_fail "--raw did not write a log file for job 1"
case "$(cat "$raw_dir/1.log" 2>/dev/null || true)" in
    *$'\033'*) note_fail "--raw left an escape sequence un-stripped" ;;
esac
case "$(cat "$raw_dir/1.log" 2>/dev/null || true)" in
    *red*) ;;
    *) note_fail "--raw lost the log content along with the escape codes" ;;
esac
export STUB_JOBS_JSON='[]'

# ── --raw needs a directory argument ────────────────────────────────────────
run_script pr/47 --raw
[ "$RUN_RC" -eq 64 ] || note_fail "--raw with no value exited $RUN_RC, expected 64"

# ── an unknown option is refused ────────────────────────────────────────────
run_script pr/47 --bogus
[ "$RUN_RC" -eq 64 ] || note_fail "an unknown option exited $RUN_RC, expected 64"

# ── missing rjq is a named, non-zero refusal ────────────────────────────────
no_rjq_bin="$work/no-rjq-bin"
mkdir -p "$no_rjq_bin"
ln -s "$stub_bin/gh" "$no_rjq_bin/gh"
ln -s "$stub_bin/glab" "$no_rjq_bin/glab"
set +e
( cd "$repo_work" && PATH="$no_rjq_bin:/usr/bin:/bin" "$BASH" "$script" pr/47 ) \
    >"$work/out" 2>"$work/err"
RUN_RC=$?
set -e
[ "$RUN_RC" -eq 69 ] || note_fail "a missing rjq exited $RUN_RC, expected 69"
case "$(cat "$work/err")" in
    *'rjq is required'*) ;;
    *) note_fail "a missing rjq was not named: $(cat "$work/err")" ;;
esac

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-ci-failures-contract: PASS'
