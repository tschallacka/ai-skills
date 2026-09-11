#!/usr/bin/env bash
# MODE: PROD
# ci-failures-glab-lib.sh — the GitLab (glab) forge backend
# ci-failures.sh dispatches to (CODE-STYLE §3, 400-line script cap).
#
# Sourced by ci-failures.sh only. Reads script-scope variables set by
# ci-failures.sh's own argument parsing (repo_override, target, format,
# raw_dir, and friends).

set -euo pipefail
export LC_ALL=C

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
