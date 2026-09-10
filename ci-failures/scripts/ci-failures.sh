#!/usr/bin/env bash
# MODE: PROD
# ci-failures - what actually failed in a CI run or pipeline, from a run/
# pipeline id, a PR/MR number, or a branch. Works against GitHub (gh) and
# GitLab (glab), detecting which one this repository's remote calls for.
#
# Usage:
#   ci-failures.sh                  # the newest run/pipeline for the current branch
#   ci-failures.sh 33894205595      # a run/pipeline id
#   ci-failures.sh 47               # a PR/MR number
#   ci-failures.sh pr/47            # a PR/MR number, unambiguously
#   ci-failures.sh fix/some-branch  # the newest run/pipeline for a branch
#   ci-failures.sh <target> --raw DIR  # keep the whole log of each failing job
#   ci-failures.sh <target> --all   # every job, not only the failing ones
#
# It prints, per failing job, the lines that identify the failure: a suite's
# own `Failed:` summary, cargo's `test result:`, every panic with the lines
# after it, GitHub's `##[error]` annotations, and this repository's own FAIL/
# portability findings. `--raw DIR` additionally writes each job's full,
# de-escaped log to DIR so a long screen dump can be read whole.
#
# WHY THIS EXISTS. Reading a failing run by hand is six steps -- list the jobs,
# find the failing ids, fetch each log through the API, allow the escape
# sequences, strip the CR and the ANSI, then search for the interesting lines
# -- and on GitHub it was done eight times in one session before anyone wrote
# it down. None of that is specific to one repository or one forge, which is
# why this is a skill rather than a maintainer note: which API answers while a
# run is still going, that GitHub's raw-log endpoint refuses a body carrying
# colour codes without an explicit flag, that the codes then need stripping
# with a literal ESC because the escape form is a GNU sed extension, and which
# lines in a log actually identify a failure as opposed to merely mentioning
# one.
#
# FORGE DETECTION. The remote this repository's origin points at decides gh or
# glab, not a flag: a github.com remote uses gh, a gitlab.com or self-hosted
# GitLab remote uses glab, and CI_FAILURES_FORGE=gh|glab overrides the guess
# outright for a remote neither pattern recognises. Either way the tool chosen
# is named on the first line of output: a silent choice between two APIs with
# different failure vocabularies is exactly the kind of guess this script
# exists to make instead of a person, and the person reading the output still
# needs to know which guess was made.
#
# BOTH FORGES, ONE VOCABULARY. GitHub calls it a run containing jobs, with a
# logs endpoint; GitLab calls it a pipeline containing jobs, with a trace
# endpoint instead, and its statuses and failure markup are not GitHub's. The
# <target> forms above (a bare number, pr/N, a branch, nothing) mean the same
# thing on both: pr/N addresses a pull request on GitHub and a merge request
# on GitLab, and a bare number is read the same way, disambiguated by
# magnitude (see resolve_run below), on both.
#
# VERIFIED DIFFERENTLY. The gh path is exercised against this repository's own
# real GitHub Actions runs. The glab path is written against GitLab's
# documented REST API v4 (pipelines, jobs, trace) -- the same stable,
# versioned surface the gh path prefers over gh's own formatted subcommands,
# and for the same reason -- but this repository has no GitLab remote to run
# it against, so it is exercised in tests against a stubbed glab rather than a
# live one. Treat a first real run against a GitLab project as the one
# still-missing verification step, not as settled.
set -euo pipefail
export LC_ALL=C

repo_override="${CI_FAILURES_REPO:-}"
forge_override="${CI_FAILURES_FORGE:-}"
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

command -v rjq >/dev/null 2>&1 || {
    printf 'ci-failures: rjq is required (run ./bootstrap.sh, or see the release page for T70)\n' >&2
    exit 69
}

# ESC as a literal byte. `\x1b` is a GNU sed extension and this repository
# targets a BSD userland too, so the pattern carries the character itself.
esc="$(printf '\033')"

# The patterns worth printing, learned from the failures this repository
# actually produces. `panicked at` pulls the four lines after it, because a
# Rust panic's message and this repo's added diagnostics -- the last screen
# rows, the wrapper's stderr, the last connect error -- are on the lines below
# the location.
extract() {
    awk -v esc="$esc" '
        { gsub(esc "\\[[0-9;]*[a-zA-Z]", ""); sub(/\r$/, "") }
        # Drop the leading ISO timestamp GitHub prefixes to every line; a
        # no-op on a GitLab trace, which carries none.
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

# ─────────────────────────────────────────────────────────────────────────────
# Forge detection: the remote decides, an explicit override is honoured
# outright, and either way the choice is named -- never silent.
# ─────────────────────────────────────────────────────────────────────────────
detect_forge() {
    if [ -n "$forge_override" ]; then
        case "$forge_override" in
            gh|glab) printf '%s\n' "$forge_override"; return ;;
            *) printf 'ci-failures: CI_FAILURES_FORGE must be gh or glab, not %s\n' "$forge_override" >&2
               exit 64 ;;
        esac
    fi
    local url
    url="$(git remote get-url origin 2>/dev/null || true)"
    case "$url" in
        *github.com*) printf 'gh\n'; return ;;
        *gitlab.com*) printf 'glab\n'; return ;;
    esac
    # A self-hosted forge names neither host, so the guess falls back to
    # whichever CLI is present and already speaks for this remote -- gh first
    # only because it was written first, not because it is preferred.
    if command -v gh >/dev/null 2>&1 && gh repo view >/dev/null 2>&1; then
        printf 'gh\n'
        return
    fi
    if command -v glab >/dev/null 2>&1 && glab repo view >/dev/null 2>&1; then
        printf 'glab\n'
        return
    fi
    printf 'ci-failures: could not tell which forge %s is; set CI_FAILURES_FORGE=gh or glab\n' \
        "${url:-this remote}" >&2
    exit 66
}

# ═════════════════════════════════════════════════════════════════════════════
# GitHub, via gh
# ═════════════════════════════════════════════════════════════════════════════
# Resolve the target to a run id.
#
# The order matters. A bare number is ambiguous -- run ids and PR numbers are
# both integers -- and the disambiguation is by MAGNITUDE, which is a
# heuristic and therefore stated out loud rather than hidden: a GitHub Actions
# run id is a 10+ digit snowflake, a PR number in this repository is two or
# three digits. `pr/47` says which is meant when that guess is not good
# enough.
gh_resolve_run() { # <repo-slug> <target> -> run id
    local repo_slug="$1" want="$2" branch head_branch
    case "$want" in
        pr/*)
            head_branch="$(gh pr view "${want#pr/}" --repo "$repo_slug" \
                --json headRefName | rjq -r .headRefName)" \
                || { printf 'ci-failures: no PR %s\n' "${want#pr/}" >&2; exit 66; }
            gh_latest_run_for "$repo_slug" "$head_branch"
            return
            ;;
        '')
            branch="$(git symbolic-ref --short -q HEAD || true)"
            [ -n "$branch" ] || { printf 'ci-failures: detached HEAD; name a run, PR or branch\n' >&2; exit 64; }
            gh_latest_run_for "$repo_slug" "$branch"
            return
            ;;
    esac
    case "$want" in
        *[!0-9]*)
            gh_latest_run_for "$repo_slug" "$want"
            return
            ;;
    esac
    if [ "${#want}" -ge 9 ]; then
        printf '%s\n' "$want"
        return
    fi
    head_branch="$(gh pr view "$want" --repo "$repo_slug" \
        --json headRefName | rjq -r .headRefName)" \
        || { printf 'ci-failures: %s is neither a run id nor a PR\n' "$want" >&2; exit 66; }
    gh_latest_run_for "$repo_slug" "$head_branch"
}

# The newest run for a branch, ignoring the artifact-render workflow: it is
# almost always green and never the reason a check suite is red.
gh_latest_run_for() { # <repo-slug> <branch> -> run id
    local repo_slug="$1" branch="$2" id
    id="$(gh run list --repo "$repo_slug" --branch "$branch" --limit 20 \
        --json databaseId,name \
        | rjq -r '[.[] | select(.name != "render-artifacts")][0].databaseId')"
    [ -n "$id" ] && [ "$id" != null ] \
        || { printf 'ci-failures: no runs for branch %s\n' "$branch" >&2; exit 66; }
    printf '%s\n' "$id"
}

# One gh job's section: header, then the failure-identifying lines extract()
# finds in its log (or the whole de-escaped log too, under --raw).
gh_print_job() { # <repo-slug> <raw-dir> <job-id> <job-conclusion> <job-name>
    local repo_slug="$1" raw_dir="$2" job_id="$3" job_conclusion="$4" job_name="$5" log found
    printf '\n== %s  [%s]  job %s\n' "$job_name" "$job_conclusion" "$job_id"
    # --allow-escape-sequences is required: gh refuses a body carrying
    # terminal colour codes, and every CI log carries them. Without it this
    # prints nothing at all and the run looks empty.
    log="$(gh api --allow-escape-sequences \
        "repos/$repo_slug/actions/jobs/$job_id/logs" 2>/dev/null || true)"
    if [ -z "$log" ]; then
        printf '    (no log; a job that never started has none)\n'
        return
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
}

run_gh() {
    command -v gh >/dev/null 2>&1 || { printf 'ci-failures: gh is required\n' >&2; exit 69; }
    local repo_slug="${repo_override:-tschallacka/ai-skills}"
    local run_id status conclusion jobs

    run_id="$(gh_resolve_run "$repo_slug" "$target")"
    status="$(gh run view "$run_id" --repo "$repo_slug" --json status | rjq -r .status)"
    conclusion="$(gh run view "$run_id" --repo "$repo_slug" --json conclusion \
        | rjq -r '.conclusion // "pending"')"
    printf 'forge: gh\nrun %s  %s/%s  https://github.com/%s/actions/runs/%s\n' \
        "$run_id" "$status" "$conclusion" "$repo_slug" "$run_id"

    # A job whose conclusion is empty is still running; `select(.conclusion ==
    # "failure")` therefore reports only settled failures, which is what "what
    # failed" means. --all keeps every job so a run can be surveyed.
    if [ "$want_all" = true ]; then
        jobs="$(gh run view "$run_id" --repo "$repo_slug" --json jobs \
            | rjq -r '.jobs[] | (.databaseId|tostring) + "\t" + (.conclusion // "running") + "\t" + .name')"
    else
        jobs="$(gh run view "$run_id" --repo "$repo_slug" --json jobs \
            | rjq -r '.jobs[] | select(.conclusion == "failure") | (.databaseId|tostring) + "\tfailure\t" + .name')"
    fi

    if [ -z "$jobs" ]; then
        if [ "$conclusion" = failure ]; then
            printf '\nno job reports failure yet, though the run does: it may still be settling,\n'
            printf 'or the failure is at the workflow level (a cancelled or skipped required job).\n'
            printf 'Re-run with --all to see every job.\n'
            return 0
        fi
        printf '\nno failing jobs.\n'
        return 0
    fi

    [ -z "$raw_dir" ] || mkdir -p "$raw_dir"
    printf '%s\n' "$jobs" | while IFS="$(printf '\t')" read -r job_id job_conclusion job_name; do
        [ -n "$job_id" ] || continue
        gh_print_job "$repo_slug" "$raw_dir" "$job_id" "$job_conclusion" "$job_name"
    done
}

# ═════════════════════════════════════════════════════════════════════════════
# GitLab, via glab
# ═════════════════════════════════════════════════════════════════════════════
# The project path, percent-encoded, for use as REST v4's :id path segment
# (GitLab accepts either the numeric project id or the URL-encoded
# "namespace/project" path there; the path is what a remote URL gives us for
# free, so that is what gets computed rather than a second lookup).
#
# `glab api`, not glab's own higher-level subcommands (`glab ci status`,
# `glab pipeline ...`): the REST endpoints are documented, versioned, and
# stable, which is exactly why the gh path above prefers `gh api` over `gh run
# view`'s own formatting too. This path has never been run against a live
# GitLab project (see VERIFIED DIFFERENTLY above) -- it is written to the
# documented contract and exercised in tests against a stub.
glab_project_path() {
    if [ -n "$repo_override" ]; then
        printf '%s\n' "$repo_override"
        return
    fi
    local url path
    url="$(git remote get-url origin 2>/dev/null || true)"
    [ -n "$url" ] || { printf 'ci-failures: no origin remote; set CI_FAILURES_REPO\n' >&2; exit 64; }
    # Two shapes, told apart by whether a scheme is present, not stripped the
    # same way: a URL form (https://host/group/sub/project.git) has the
    # project path after the FIRST slash past the host, while an scp-like
    # form (git@host:group/sub/project.git) has no host slash to strip at all
    # -- applying both stripping steps to the scp-like form ate its first
    # path segment as if it were a host.
    case "$url" in
        *://*)
            path="${url#*://}"
            path="${path#*/}"
            ;;
        *)
            path="${url#*@}"
            path="${path#*:}"
            ;;
    esac
    path="${path%.git}"
    printf '%s\n' "$path"
}

glab_encoded_project() {
    glab_project_path | sed 's|/|%2F|g'
}

# The newest pipeline for a branch.
glab_latest_pipeline_for() { # <branch> -> pipeline id
    local branch="$1" project id
    project="$(glab_encoded_project)"
    id="$(glab api "projects/$project/pipelines?ref=$branch&order_by=id&sort=desc&per_page=1" \
        | rjq -r '.[0].id // empty')"
    [ -n "$id" ] || { printf 'ci-failures: no pipelines for branch %s\n' "$branch" >&2; exit 66; }
    printf '%s\n' "$id"
}

# Resolution mirrors gh_resolve_run: pr/N and a bare small number both name a
# merge request (GitLab's MR !iid), a bare large number names a pipeline id
# directly, empty names the current branch. The magnitude threshold is the
# same heuristic as the gh path, for the same reason: stated out loud, not
# hidden, and `pr/N` (kept as the spelling on both forges, rather than asking
# for `mr/N` here) says which is meant when the guess is not good enough.
glab_resolve_pipeline() { # <target> -> pipeline id
    local want="$1" project branch head_branch
    project="$(glab_encoded_project)"
    case "$want" in
        pr/*)
            head_branch="$(glab api "projects/$project/merge_requests/${want#pr/}" \
                | rjq -r .source_branch)" \
                || { printf 'ci-failures: no MR %s\n' "${want#pr/}" >&2; exit 66; }
            glab_latest_pipeline_for "$head_branch"
            return
            ;;
        '')
            branch="$(git symbolic-ref --short -q HEAD || true)"
            [ -n "$branch" ] || { printf 'ci-failures: detached HEAD; name a pipeline, MR or branch\n' >&2; exit 64; }
            glab_latest_pipeline_for "$branch"
            return
            ;;
    esac
    case "$want" in
        *[!0-9]*)
            glab_latest_pipeline_for "$want"
            return
            ;;
    esac
    if [ "${#want}" -ge 9 ]; then
        printf '%s\n' "$want"
        return
    fi
    head_branch="$(glab api "projects/$project/merge_requests/$want" \
        | rjq -r .source_branch)" \
        || { printf 'ci-failures: %s is neither a pipeline id nor an MR\n' "$want" >&2; exit 66; }
    glab_latest_pipeline_for "$head_branch"
}

# One glab job's section, mirroring gh_print_job: header, then extract()'s
# findings (or the whole de-escaped log too, under --raw).
glab_print_job() { # <project> <raw-dir> <job-id> <job-status> <job-name>
    local project="$1" raw_dir="$2" job_id="$3" job_status="$4" job_name="$5" log found
    printf '\n== %s  [%s]  job %s\n' "$job_name" "$job_status" "$job_id"
    # The trace endpoint returns the raw job log as plain text (not JSON),
    # unlike every other call here -- piped straight through, not rjq.
    log="$(glab api "projects/$project/jobs/$job_id/trace" 2>/dev/null || true)"
    if [ -z "$log" ]; then
        printf '    (no log; a job that never started has none)\n'
        return
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
}

run_glab() {
    command -v glab >/dev/null 2>&1 || { printf 'ci-failures: glab is required\n' >&2; exit 69; }
    local project pipeline_id status jobs
    project="$(glab_encoded_project)"
    pipeline_id="$(glab_resolve_pipeline "$target")"

    status="$(glab api "projects/$project/pipelines/$pipeline_id" | rjq -r .status)"
    printf 'forge: glab\npipeline %s  %s  https://gitlab.com/%s/-/pipelines/%s\n' \
        "$pipeline_id" "$status" "$(glab_project_path)" "$pipeline_id"

    if [ "$want_all" = true ]; then
        jobs="$(glab api "projects/$project/pipelines/$pipeline_id/jobs?per_page=100" \
            | rjq -r '.[] | (.id|tostring) + "\t" + .status + "\t" + .name')"
    else
        jobs="$(glab api "projects/$project/pipelines/$pipeline_id/jobs?per_page=100" \
            | rjq -r '.[] | select(.status == "failed") | (.id|tostring) + "\tfailed\t" + .name')"
    fi

    if [ -z "$jobs" ]; then
        if [ "$status" = failed ]; then
            printf '\nno job reports failed yet, though the pipeline does: it may still be\n'
            printf 'settling, or the failure is at the pipeline level. Re-run with --all to\n'
            printf 'see every job.\n'
            return 0
        fi
        printf '\nno failing jobs.\n'
        return 0
    fi

    [ -z "$raw_dir" ] || mkdir -p "$raw_dir"
    printf '%s\n' "$jobs" | while IFS="$(printf '\t')" read -r job_id job_status job_name; do
        [ -n "$job_id" ] || continue
        glab_print_job "$project" "$raw_dir" "$job_id" "$job_status" "$job_name"
    done
}

forge="$(detect_forge)"
case "$forge" in
    gh) run_gh ;;
    glab) run_glab ;;
esac
