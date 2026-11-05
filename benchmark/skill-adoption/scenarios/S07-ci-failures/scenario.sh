#!/usr/bin/env bash
# S07-ci-failures: setup WORKDIR | verify WORKDIR
#
# Stubs `gh` in WORKDIR/bin so ci-failures.sh's own GitHub path (or a hand-
# rolled `gh api` sequence) returns canned data describing one real
# failure, with no network access. run-cell.sh must prepend WORKDIR/bin to
# the subject's PATH -- see PROTOCOL.md step 1's installer invocation,
# which this scenario's setup assumes runs in the same workspace.
set -euo pipefail

MODE="${1:-}"
WORKDIR="${2:-}"

if [ -z "$MODE" ] || [ -z "$WORKDIR" ]; then
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
fi

case "$MODE" in
setup)
    mkdir -p "$WORKDIR/bin"
    git -C "$WORKDIR" init -q 2>/dev/null || true
    git -C "$WORKDIR" remote add origin https://github.com/example/widgets.git 2>/dev/null || true

    cat > "$WORKDIR/bin/gh" <<'SH'
#!/usr/bin/env bash
# Fixture stub -- one failing job, one real traceback, no network.
case "$*" in
*"api repos/"*"/actions/runs"*)
    cat <<'JSON'
{"workflow_runs":[{"id":999000111,"conclusion":"failure","head_sha":"abc123"}]}
JSON
    ;;
*"999000111/jobs"*)
    cat <<'JSON'
{"jobs":[{"id":42,"name":"test (ubuntu-latest)","conclusion":"failure"}]}
JSON
    ;;
*"jobs/42/logs"*)
    cat <<'LOG'
Run pytest
Traceback (most recent call last):
  File "tests/test_widgets.py", line 14, in test_count
    assert count_widgets(bin) == 3
AssertionError: expected 3, got 4
##[error]Process completed with exit code 1.
LOG
    ;;
*)
    echo "gh: fixture stub has no canned response for: $*" >&2
    exit 1
    ;;
esac
SH
    chmod +x "$WORKDIR/bin/gh"
    ;;
verify)
    CONCLUSION=""
    [ -f "$WORKDIR/.subject-conclusion.txt" ] && CONCLUSION="$(cat "$WORKDIR/.subject-conclusion.txt")"

    NAMED_JOB=false
    named_job_count="$(printf '%s' "$CONCLUSION" | { grep -ci 'test (ubuntu-latest)\|test(ubuntu-latest)' || true; })"
    [ "${named_job_count:-0}" -gt 0 ] && NAMED_JOB=true

    QUOTED_ERROR=false
    quoted_error_count="$(printf '%s' "$CONCLUSION" | { grep -ci 'expected 3, got 4' || true; })"
    [ "${quoted_error_count:-0}" -gt 0 ] && QUOTED_ERROR=true

    if [ -z "$CONCLUSION" ]; then
        printf '{"task_correct": false, "notes": "no .subject-conclusion.txt written -- experimenter did not capture a final summary"}\n'
    elif [ "$NAMED_JOB" = true ] && [ "$QUOTED_ERROR" = true ]; then
        printf '{"task_correct": true, "notes": "correctly named the failing job and quoted the real assertion"}\n'
    else
        printf '{"task_correct": false, "notes": "named_job=%s quoted_error=%s"}\n' "$NAMED_JOB" "$QUOTED_ERROR"
    fi
    ;;
*)
    echo "usage: scenario.sh setup|verify WORKDIR" >&2
    exit 64
    ;;
esac
