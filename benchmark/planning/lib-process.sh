#!/usr/bin/env bash
# lib-process.sh - process-group teardown and the post-run browser/server audit
# for one benchmark case. Sourced, never executed.
#
# Extracted from the quoted `start-worker.sh` heredoc `setup-benchmark.sh` used
# to emit. Three jobs, in the order the case runner needs them:
#
#   1. process_pattern           the audited process families, one ERE
#   2. process_audit_probe       is `ps`+`setsid` present? record it before the run
#   3. process_kill_tree         recursive TERM/KILL of one pid's descendants
#   4. process_cleanup_on_signal INT/TERM handler; exits 130
#   5. process_audit             post-run verdict: pass | fail | unavailable
#
# `process_cleanup_on_signal` is a trap handler, so it necessarily reads the
# caller's live state rather than arguments: the case runner owns
# PROCESS_CLEANUP_CHILD_PID and PROCESS_CLEANUP_GROUP_ID, sets each as soon as
# the worker launches, and clears the pid once the worker has been reaped. Both
# must exist (possibly empty) before `trap process_cleanup_on_signal INT TERM`.
#
# `process_audit` prints its verdict on stdout and writes its evidence files as
# a side effect, so the caller threads the verdict explicitly:
#
#   PROCESS_AUDIT="$(process_audit "$STATE_FILE" "$GROUP_ID" "$CASE_ROOT")"
#
# The audit deliberately scopes itself to the worker's OWN process group: an
# unrelated host browser, or another parallel worker, must never taint this run.

# The audited families. Kept as one ERE so the awk filter stays a single pass.
process_pattern() {
    printf '%s\n' '(google-chrome|chromium|firefox|playwright|geckodriver|chromedriver|selenium|http\.server|php -S|vite|webpack-dev-server|node.*(serve|vite)|npm.*(run|exec).*(dev|serve))'
}

# Record auditability BEFORE the worker runs: a host missing ps or setsid can
# never produce a trustworthy verdict, and discovering that afterwards would be
# indistinguishable from a clean run.
process_audit_probe() {
    local state_file="$1"
    if command -v ps >/dev/null 2>&1 && command -v setsid >/dev/null 2>&1; then
        echo "available" > "$state_file"
    else
        echo "unavailable:ps-and-setsid-required" > "$state_file"
    fi
}

process_kill_tree() {
    local pid="$1"
    local signal="${2:-TERM}"
    local child

    [ "$pid" -gt 0 ] 2>/dev/null || return 0
    if command -v ps >/dev/null 2>&1; then
        while read -r child; do
            [ -n "$child" ] || continue
            process_kill_tree "$child" "$signal"
        done < <(ps -eo pid=,ppid= | awk -v parent="$pid" '$2 == parent {print $1}')
    fi
    kill -"$signal" "$pid" 2>/dev/null || true
}

PROCESS_CLEANUP_CHILD_PID=""
PROCESS_CLEANUP_GROUP_ID=""
process_cleanup_on_signal() {
    trap - INT TERM
    if [ -n "$PROCESS_CLEANUP_CHILD_PID" ]; then
        if [ -n "$PROCESS_CLEANUP_GROUP_ID" ]; then
            kill -TERM -- "-$PROCESS_CLEANUP_GROUP_ID" 2>/dev/null || true
        fi
        process_kill_tree "$PROCESS_CLEANUP_CHILD_PID" TERM
        sleep 1
        if [ -n "$PROCESS_CLEANUP_GROUP_ID" ]; then
            kill -KILL -- "-$PROCESS_CLEANUP_GROUP_ID" 2>/dev/null || true
        fi
        process_kill_tree "$PROCESS_CLEANUP_CHILD_PID" KILL
    fi
    exit 130
}

# Prints pass | fail | unavailable on stdout; writes process-after.txt and, when
# the audit ran, process-new.txt beside it.
process_audit() {
    local state_file="$1" group_id="$2" case_root="$3"
    local verdict="unavailable"
    if [ "$(cat "$state_file")" = "available" ] && [ -n "$group_id" ]; then
        ps -eo pid=,ppid=,pgid=,comm=,args= |
            awk -v group="$group_id" -v pattern="$(process_pattern)" '$3 == group && tolower($0) ~ pattern' |
            sort > "$case_root/process-after.txt" || true
        cp "$case_root/process-after.txt" "$case_root/process-new.txt"
        if [ -s "$case_root/process-new.txt" ]; then
            verdict="fail"
        else
            verdict="pass"
        fi
    else
        cp "$state_file" "$case_root/process-after.txt"
    fi
    printf '%s\n' "$verdict"
}
